use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{OutputFormat, print_error, print_result};
use crate::utils::redact_url_password;
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct StepSummary {
    step_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct StatusResult {
    run_id: String,
    workflow_id: String,
    status: String,
    steps_pending: usize,
    steps_running: usize,
    steps_succeeded: usize,
    steps_failed: usize,
    steps_skipped: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failed_steps: Vec<StepSummary>,
}

pub async fn status_cmd(run_id: &str, output: OutputArgs, store: StoreArgs) -> i32 {
    let run_uuid = match Uuid::parse_str(run_id) {
        Ok(u) => u,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("invalid run_id: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let database_url = match store.store
        .or_else(|| std::env::var("ARAZZO_DATABASE_URL").ok())
        .or_else(|| std::env::var("DATABASE_URL").ok())
    {
        Some(v) => v,
        None => {
            print_error(output.format, output.quiet, "missing database URL");
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let pg = match arazzo_store::PostgresStore::connect(&database_url, 5).await {
        Ok(s) => s,
        Err(e) => {
            let safe_url = redact_url_password(&database_url);
            print_error(output.format, output.quiet, &format!("database connection failed to {}: {e}. Check your DATABASE_URL and ensure Postgres is running.", safe_url));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let run = match pg.get_run(run_uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            print_error(output.format, output.quiet, "run not found");
            return exit_codes::RUNTIME_ERROR;
        }
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to get run {}: {e}. Run may not exist or database error occurred.", run_uuid));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let steps = match pg.get_run_steps(run_uuid).await {
        Ok(s) => s,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to get steps: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let mut pending = 0;
    let mut running = 0;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failed_steps = Vec::new();

    for step in &steps {
        match step.status.as_str() {
            "pending" => pending += 1,
            "running" => running += 1,
            "succeeded" => succeeded += 1,
            "failed" => {
                failed += 1;
                failed_steps.push(StepSummary {
                    step_id: step.step_id.clone(),
                    status: step.status.clone(),
                    error: step.error.as_ref().and_then(|e| e.get("message").and_then(|m| m.as_str()).map(String::from)),
                });
            }
            "skipped" => skipped += 1,
            _ => {}
        }
    }

    let result = StatusResult {
        run_id: run_uuid.to_string(),
        workflow_id: run.workflow_id.clone(),
        status: run.status.clone(),
        steps_pending: pending,
        steps_running: running,
        steps_succeeded: succeeded,
        steps_failed: failed,
        steps_skipped: skipped,
        failed_steps,
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Run: {}", result.run_id);
        println!("Workflow: {}", result.workflow_id);
        println!("Status: {}", result.status);
        println!();
        println!("Steps:");
        println!("  Pending:   {}", result.steps_pending);
        println!("  Running:   {}", result.steps_running);
        println!("  Succeeded: {}", result.steps_succeeded);
        println!("  Failed:    {}", result.steps_failed);
        println!("  Skipped:   {}", result.steps_skipped);
        if !result.failed_steps.is_empty() {
            println!();
            println!("Failed steps:");
            for fs in &result.failed_steps {
                print!("  - {}", fs.step_id);
                if let Some(e) = &fs.error {
                    print!(": {e}");
                }
                println!();
            }
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}


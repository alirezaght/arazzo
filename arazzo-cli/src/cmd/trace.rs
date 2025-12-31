use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::utils::redact_url_password;
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct AttemptInfo {
    attempt_no: i32,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct StepTrace {
    step_id: String,
    step_index: i32,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attempts: Vec<AttemptInfo>,
}

#[derive(Serialize)]
struct TraceResult {
    run_id: String,
    workflow_id: String,
    status: String,
    steps: Vec<StepTrace>,
}

pub async fn trace_cmd(run_id: &str, output: OutputArgs, store: StoreArgs) -> i32 {
    let run_uuid = match Uuid::parse_str(run_id) {
        Ok(u) => u,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("invalid run_id: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let database_url = match store
        .store
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
            print_error(
                output.format,
                output.quiet,
                &format!(
                    "failed to get run {}: {e}. Run may not exist or database error occurred.",
                    run_uuid
                ),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let steps = match pg.get_run_steps(run_uuid).await {
        Ok(s) => s,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to get steps: {e}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let mut step_traces = Vec::new();
    for step in &steps {
        let attempts = pg.get_step_attempts(step.id).await.unwrap_or_default();

        let attempt_infos: Vec<AttemptInfo> = attempts
            .iter()
            .map(|a| AttemptInfo {
                attempt_no: a.attempt_no,
                status: a.status.clone(),
                duration_ms: a.duration_ms,
                error: a
                    .error
                    .as_ref()
                    .and_then(|e| e.get("message").and_then(|m| m.as_str()).map(String::from)),
            })
            .collect();

        step_traces.push(StepTrace {
            step_id: step.step_id.clone(),
            step_index: step.step_index,
            status: step.status.clone(),
            depends_on: step.depends_on.clone(),
            attempts: attempt_infos,
        });
    }

    step_traces.sort_by_key(|s| s.step_index);

    let result = TraceResult {
        run_id: run_uuid.to_string(),
        workflow_id: run.workflow_id.clone(),
        status: run.status.clone(),
        steps: step_traces,
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Run: {} ({})", result.run_id, result.status);
        println!("Workflow: {}", result.workflow_id);
        println!();
        for s in &result.steps {
            let deps = if s.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (deps: {})", s.depends_on.join(", "))
            };
            println!(
                "Step {}: {} [{}]{}",
                s.step_index, s.step_id, s.status, deps
            );
            for a in &s.attempts {
                let dur = a
                    .duration_ms
                    .map(|d| format!(" {}ms", d))
                    .unwrap_or_default();
                let err = a
                    .error
                    .as_ref()
                    .map(|e| format!(" - {e}"))
                    .unwrap_or_default();
                println!("  Attempt {}: {}{}{}", a.attempt_no, a.status, dur, err);
            }
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}

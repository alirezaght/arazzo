use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{OutputFormat, print_error, print_result};
use crate::utils::redact_url_password;
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct MetricsResult {
    run_id: String,
    workflow_id: String,
    status: String,
    duration_ms: Option<u64>,
    steps: StepMetrics,
    http: HttpMetrics,
    policy_denials: usize,
}

#[derive(Serialize)]
struct StepMetrics {
    total: usize,
    succeeded: usize,
    failed: usize,
    retried: usize,
}

#[derive(Serialize)]
struct HttpMetrics {
    requests: usize,
    errors: usize,
}

pub async fn metrics_cmd(run_id: &str, output: OutputArgs, store: StoreArgs) -> i32 {
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
            print_error(output.format, output.quiet, &format!("run {} not found", run_uuid));
            return exit_codes::RUNTIME_ERROR;
        }
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to get run: {e}"));
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

    let mut succeeded = 0;
    let mut failed = 0;
    let mut retried = 0;
    let mut http_requests = 0;
    let mut http_errors = 0;
    let mut policy_denials = 0;

    for step in &steps {
        match step.status.as_str() {
            "succeeded" => succeeded += 1,
            "failed" => failed += 1,
            _ => {}
        }
    }

    let events = match pg.get_events_after(run_uuid, 0, 10000).await {
        Ok(e) => e,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to get events: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    for event in &events {
        match event.event_type.as_str() {
            "attempt.started" => http_requests += 1,
            "attempt.finished" => {
                if let Some(succeeded_val) = event.payload.get("succeeded") {
                    if !succeeded_val.as_bool().unwrap_or(false) {
                        http_errors += 1;
                    }
                }
            }
            "step.retry_scheduled" => retried += 1,
            "policy.denied" => policy_denials += 1,
            _ => {}
        }
    }

    let duration_ms = if let (Some(started), Some(finished)) = (run.started_at, run.finished_at) {
        Some(finished.signed_duration_since(started).num_milliseconds() as u64)
    } else {
        None
    };

    let result = MetricsResult {
        run_id: run_uuid.to_string(),
        workflow_id: run.workflow_id,
        status: run.status,
        duration_ms,
        steps: StepMetrics {
            total: steps.len(),
            succeeded,
            failed,
            retried,
        },
        http: HttpMetrics {
            requests: http_requests,
            errors: http_errors,
        },
        policy_denials,
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Metrics for run {}", run_uuid);
        println!("  Workflow: {}", result.workflow_id);
        println!("  Status: {}", result.status);
        if let Some(duration) = result.duration_ms {
            println!("  Duration: {}ms ({:.2}s)", duration, duration as f64 / 1000.0);
        }
        println!("  Steps: {}/{} succeeded, {} failed, {} retried", 
            result.steps.succeeded, result.steps.total, result.steps.failed, result.steps.retried);
        println!("  HTTP: {} requests, {} errors", result.http.requests, result.http.errors);
        println!("  Policy denials: {}", result.policy_denials);
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}


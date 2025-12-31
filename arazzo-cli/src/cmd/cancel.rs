use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{OutputFormat, print_error, print_result};
use crate::utils::redact_url_password;
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct CancelResult {
    run_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_status: Option<String>,
}

pub async fn cancel_cmd(run_id: &str, output: OutputArgs, store: StoreArgs) -> i32 {
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

    let previous_status = run.status.clone();

    if previous_status == "canceled" {
        let result = CancelResult {
            run_id: run_uuid.to_string(),
            status: "canceled".to_string(),
            previous_status: Some(previous_status),
        };
        if output.format == OutputFormat::Text && !output.quiet {
            println!("Run {} already canceled", run_uuid);
        } else {
            print_result(output.format, output.quiet, &result);
        }
        return exit_codes::SUCCESS;
    }

    if previous_status == "succeeded" || previous_status == "failed" {
        print_error(output.format, output.quiet, &format!("run already in terminal state: {previous_status}"));
        return exit_codes::RUNTIME_ERROR;
    }

    if let Err(e) = pg.mark_run_finished(run_uuid, "canceled", None).await {
        print_error(output.format, output.quiet, &format!("failed to cancel run: {e}"));
        return exit_codes::RUNTIME_ERROR;
    }

    let result = CancelResult {
        run_id: run_uuid.to_string(),
        status: "canceled".to_string(),
        previous_status: Some(previous_status),
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Run {} canceled", run_uuid);
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}


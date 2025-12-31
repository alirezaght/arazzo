use serde::Serialize;

use arazzo_store::{PostgresStore, run_migrations};

use crate::exit_codes;
use crate::output::{OutputFormat, print_error, print_result};
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct MigrateResult {
    success: bool,
    message: String,
}

pub async fn migrate_cmd(store: StoreArgs, max_connections: u32, output: OutputArgs) -> i32 {
    let database_url = match store.store
        .or_else(|| std::env::var("ARAZZO_DATABASE_URL").ok())
        .or_else(|| std::env::var("DATABASE_URL").ok())
    {
        Some(v) => v,
        None => {
            print_error(output.format, output.quiet, "missing database url (use --store or set ARAZZO_DATABASE_URL / DATABASE_URL)");
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let pg = match PostgresStore::connect(&database_url, max_connections).await {
        Ok(s) => s,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to connect to postgres: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    match run_migrations(pg.pool()).await {
        Ok(()) => {
            let result = MigrateResult {
                success: true,
                message: "migrations applied".to_string(),
            };
            if output.format == OutputFormat::Text && !output.quiet {
                println!("ok: migrations applied");
            } else {
                print_result(output.format, output.quiet, &result);
            }
            exit_codes::SUCCESS
        }
        Err(e) => {
            print_error(output.format, output.quiet, &format!("migration failed: {e}"));
            exit_codes::RUNTIME_ERROR
        }
    }
}

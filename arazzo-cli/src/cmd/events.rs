use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::utils::redact_url_password;
use crate::{OutputArgs, StoreArgs};

#[derive(Serialize)]
struct EventInfo {
    id: i64,
    ts: String,
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    step_id: Option<String>,
    payload: serde_json::Value,
}

pub async fn events_cmd(run_id: &str, follow: bool, output: OutputArgs, store: StoreArgs) -> i32 {
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

    let step_id_map: std::collections::HashMap<Uuid, String> =
        steps.iter().map(|s| (s.id, s.step_id.clone())).collect();

    let mut last_id: i64 = 0;

    loop {
        let events = match pg.get_events_after(run_uuid, last_id, 100).await {
            Ok(e) => e,
            Err(e) => {
                print_error(
                    output.format,
                    output.quiet,
                    &format!("failed to get events: {e}"),
                );
                return exit_codes::RUNTIME_ERROR;
            }
        };

        if events.is_empty() {
            if !follow {
                break;
            }
            if let Ok(Some(run)) = pg.get_run(run_uuid).await {
                if matches!(run.status.as_str(), "succeeded" | "failed" | "canceled") {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    let final_events = pg
                        .get_events_after(run_uuid, last_id, 100)
                        .await
                        .unwrap_or_default();
                    for event in &final_events {
                        let step_id = event
                            .run_step_id
                            .and_then(|id| step_id_map.get(&id).cloned());
                        let info = EventInfo {
                            id: event.id,
                            ts: event.ts.to_rfc3339(),
                            r#type: event.event_type.clone(),
                            step_id,
                            payload: event.payload.clone(),
                        };
                        if output.format == OutputFormat::Text && !output.quiet {
                            let step_str = info
                                .step_id
                                .as_ref()
                                .map(|s| format!(" [{}]", s))
                                .unwrap_or_default();
                            println!("{} {}{}", info.ts, info.r#type, step_str);
                            if !info.payload.is_null() && info.payload != serde_json::json!({}) {
                                if let Ok(s) = serde_json::to_string(&info.payload) {
                                    println!("  {s}");
                                }
                            }
                        } else {
                            print_result(output.format, output.quiet, &info);
                        }
                    }
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            continue;
        }

        for event in &events {
            last_id = event.id;
            let step_id = event
                .run_step_id
                .and_then(|id| step_id_map.get(&id).cloned());

            let info = EventInfo {
                id: event.id,
                ts: event.ts.to_rfc3339(),
                r#type: event.event_type.clone(),
                step_id,
                payload: event.payload.clone(),
            };

            if output.format == OutputFormat::Text && !output.quiet {
                let step_str = info
                    .step_id
                    .as_ref()
                    .map(|s| format!(" [{}]", s))
                    .unwrap_or_default();
                println!("{} {}{}", info.ts, info.r#type, step_str);
                if !info.payload.is_null() && info.payload != serde_json::json!({}) {
                    if let Ok(s) = serde_json::to_string(&info.payload) {
                        println!("  {s}");
                    }
                }
            } else {
                print_result(output.format, output.quiet, &info);
            }
        }

        if !follow {
            break;
        }
    }

    exit_codes::SUCCESS
}

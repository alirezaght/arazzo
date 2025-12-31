use std::sync::Arc;

use arazzo_core::{DocumentFormat, PlanOptions, parse_document_str, plan_document};
#[allow(unused_imports)]
use arazzo_store::StateStore;
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{OutputFormat, print_error, print_result};
use crate::{ConcurrencyArgs, OutputArgs, PolicyArgs, RetryArgs, SecretsArgs, StoreArgs};

use super::config::{build_executor_config, build_policy_config, get_database_url};
use crate::utils::redact_url_password;

#[derive(Serialize)]
struct ResumeResult {
    run_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    steps_succeeded: usize,
    steps_failed: usize,
}

pub async fn resume_cmd(
    run_id: &str,
    output: OutputArgs,
    store: StoreArgs,
    _secrets: SecretsArgs,
    policy: PolicyArgs,
    concurrency: ConcurrencyArgs,
    retry: RetryArgs,
) -> i32 {
    let run_uuid = match Uuid::parse_str(run_id) {
        Ok(u) => u,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("invalid run_id: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let database_url = match get_database_url(store.store, &output) {
        Some(v) => v,
        None => return exit_codes::RUNTIME_ERROR,
    };

    let pg = match arazzo_store::PostgresStore::connect(&database_url, 10).await {
        Ok(s) => s,
        Err(e) => {
            let safe_url = redact_url_password(&database_url);
            print_error(output.format, output.quiet, &format!("database connection failed to {}: {e}. Check your DATABASE_URL and ensure Postgres is running.", safe_url));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let store_arc: Arc<dyn arazzo_store::StateStore> = Arc::new(pg);

    let run = match store_arc.get_run(run_uuid).await {
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

    if run.status == "succeeded" || run.status == "failed" || run.status == "canceled" {
        let result = ResumeResult {
            run_id: run_uuid.to_string(),
            status: run.status.clone(),
            error: Some(format!("run already in terminal state: {}", run.status)),
            steps_succeeded: 0,
            steps_failed: 0,
        };
        if output.format == OutputFormat::Text && !output.quiet {
            eprintln!("Run {} already in terminal state: {}", run_uuid, run.status);
        } else {
            print_result(output.format, output.quiet, &result);
        }
        return exit_codes::RUNTIME_ERROR;
    }

    let workflow_doc = match store_arc.get_workflow_doc(run.workflow_doc_id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            print_error(output.format, output.quiet, "workflow document not found");
            return exit_codes::RUNTIME_ERROR;
        }
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to get workflow doc: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let format = match workflow_doc.format.as_str() {
        "json" => DocumentFormat::Json,
        _ => DocumentFormat::Yaml,
    };
    let parsed = match parse_document_str(&workflow_doc.raw, format) {
        Ok(p) => p,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to parse workflow: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let inputs: Option<serde_json::Value> = if run.inputs.is_null() {
        None
    } else {
        Some(run.inputs.clone())
    };

    let outcome = match plan_document(&parsed.document, PlanOptions {
        workflow_id: Some(run.workflow_id.clone()),
        inputs: inputs.clone(),
    }) {
        Ok(o) => o,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to plan: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    if !outcome.validation.is_valid {
        print_error(output.format, output.quiet, "workflow validation failed");
        return exit_codes::VALIDATION_FAILED;
    }

    let plan = match &outcome.plan {
        Some(p) => p,
        None => {
            print_error(output.format, output.quiet, "no plan generated");
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let wf = match parsed.document.workflows.iter().find(|w| w.workflow_id == plan.summary.workflow_id) {
        Some(w) => w,
        None => {
            print_error(output.format, output.quiet, "workflow not found in document");
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let compiled = arazzo_exec::Compiler::default().compile_workflow(&parsed.document, wf).await;
    if compiled.diagnostics.iter().any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error) {
        print_error(output.format, output.quiet, "OpenAPI compilation failed");
        return exit_codes::VALIDATION_FAILED;
    }

    let exec_config = build_executor_config(&concurrency, &retry);
    let secrets_provider: Arc<dyn arazzo_exec::secrets::SecretsProvider> =
        Arc::new(arazzo_exec::secrets::EnvSecretsProvider::default());
    let policy_gate = Arc::new(arazzo_exec::policy::PolicyGate::new(build_policy_config(&policy)));
    let http_client: Arc<dyn arazzo_exec::executor::HttpClient> =
        Arc::new(arazzo_exec::executor::http::ReqwestHttpClient::default());
    let event_sink: Arc<dyn arazzo_exec::executor::EventSink> =
        Arc::new(arazzo_exec::executor::StoreEventSink::new(store_arc.clone()));

    let executor = arazzo_exec::Executor::new(
        exec_config, store_arc.clone(), http_client, secrets_provider, policy_gate, event_sink,
    );

    let run_inputs = inputs.unwrap_or(serde_json::json!({}));

    // Reset any steps stuck in 'running' state from a previous crash
    match store_arc.reset_stale_running_steps(run_uuid).await {
        Ok(count) if count > 0 => {
            if output.format == OutputFormat::Text && !output.quiet {
                println!("Reset {} stale running step(s)", count);
            }
        }
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to reset stale steps: {e}"));
            return exit_codes::RUNTIME_ERROR;
        }
        _ => {}
    }

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Resuming run {}...", run_uuid);
    }

    let result = executor.execute_run(run_uuid, wf, &compiled, &run_inputs, Some(&parsed.document)).await;

    match result {
        Ok(exec_result) => {
            let res = ResumeResult {
                run_id: run_uuid.to_string(),
                status: "succeeded".to_string(),
                error: None,
                steps_succeeded: exec_result.succeeded_steps,
                steps_failed: exec_result.failed_steps,
            };
            if output.format == OutputFormat::Text && !output.quiet {
                println!("Run {} completed", run_uuid);
                println!("  Steps succeeded: {}", res.steps_succeeded);
                println!("  Steps failed: {}", res.steps_failed);
            } else {
                print_result(output.format, output.quiet, &res);
            }
            if res.steps_failed > 0 { exit_codes::RUN_FAILED } else { exit_codes::SUCCESS }
        }
        Err(e) => {
            let res = ResumeResult {
                run_id: run_uuid.to_string(),
                status: "failed".to_string(),
                error: Some(format!("{e:?}")),
                steps_succeeded: 0,
                steps_failed: 0,
            };
            if output.format == OutputFormat::Text && !output.quiet {
                eprintln!("Run {} failed: {:?}", run_uuid, e);
            } else {
                print_result(output.format, output.quiet, &res);
            }
            exit_codes::RUN_FAILED
        }
    }
}

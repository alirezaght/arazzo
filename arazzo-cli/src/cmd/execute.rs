use std::path::Path;
use std::sync::Arc;

use arazzo_core::{parse_document_str, plan_document, DocumentFormat, PlanOptions};
use serde::Serialize;
use uuid::Uuid;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::{
    ConcurrencyArgs, OpenApiArgs, OutputArgs, PolicyArgs, RetryArgs, SecretsArgs, StoreArgs,
};

use super::config::{
    build_executor_config, build_policy_config, get_database_url, load_inputs, merge_set_inputs,
};
use crate::utils::redact_url_password;

#[derive(Serialize)]
struct ExecuteResult {
    run_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    steps_succeeded: usize,
    steps_failed: usize,
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_cmd(
    path: &Path,
    workflow_id: Option<&str>,
    inputs_path: Option<&Path>,
    set_inputs: &[String],
    run_id: Option<&str>,
    idempotency_key: Option<&str>,
    events: &str,
    output: OutputArgs,
    store: StoreArgs,
    _openapi: OpenApiArgs,
    _secrets: SecretsArgs,
    webhook: crate::WebhookArgs,
    policy: PolicyArgs,
    concurrency: ConcurrencyArgs,
    retry: RetryArgs,
) -> i32 {
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to read {}: {e}", path.display()),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let parsed = match parse_document_str(&content, DocumentFormat::Auto) {
        Ok(p) => p,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("{e}"));
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let mut inputs = load_inputs(inputs_path, &output);
    if inputs.is_none() && inputs_path.is_some() {
        return exit_codes::RUNTIME_ERROR;
    }
    merge_set_inputs(&mut inputs, set_inputs);

    let outcome = match plan_document(
        &parsed.document,
        PlanOptions {
            workflow_id: workflow_id.map(String::from),
            inputs: inputs.clone(),
        },
    ) {
        Ok(o) => o,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("{e}"));
            return exit_codes::VALIDATION_FAILED;
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

    let wf = match parsed
        .document
        .workflows
        .iter()
        .find(|w| w.workflow_id == plan.summary.workflow_id)
    {
        Some(w) => w,
        None => {
            print_error(output.format, output.quiet, "workflow not found");
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let compiled = arazzo_exec::Compiler::default()
        .compile_workflow(&parsed.document, wf)
        .await;
    if compiled
        .diagnostics
        .iter()
        .any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error)
    {
        print_error(output.format, output.quiet, "OpenAPI compilation failed");
        return exit_codes::VALIDATION_FAILED;
    }

    let database_url = match get_database_url(store.store, &output) {
        Some(u) => u,
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

    if let Some(id) = run_id {
        if Uuid::parse_str(id).is_err() {
            print_error(
                output.format,
                output.quiet,
                &format!("invalid run_id: {id}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    }

    let exec_config = build_executor_config(&concurrency, &retry);
    let secrets_provider: Arc<dyn arazzo_exec::secrets::SecretsProvider> =
        Arc::new(arazzo_exec::secrets::EnvSecretsProvider::default());
    let policy_gate = Arc::new(arazzo_exec::policy::PolicyGate::new(build_policy_config(
        &policy,
    )));
    let http_client: Arc<dyn arazzo_exec::executor::HttpClient> =
        Arc::new(arazzo_exec::executor::http::ReqwestHttpClient::default());
    let store_arc: Arc<dyn arazzo_store::StateStore> = Arc::new(pg);

    let total_steps = plan.steps.len();
    let show_progress = output.format == OutputFormat::Text && !output.quiet;
    let progress_sink: Option<Arc<super::progress::ProgressEventSink>> = if show_progress {
        Some(Arc::new(super::progress::ProgressEventSink::new(
            total_steps,
        )))
    } else {
        None
    };

    let base_event_sink: Arc<dyn arazzo_exec::executor::EventSink> = match events {
        "none" => Arc::new(arazzo_exec::executor::NoOpEventSink),
        "stdout" => Arc::new(arazzo_exec::executor::StdoutEventSink),
        "postgres" => Arc::new(arazzo_exec::executor::StoreEventSink::new(
            store_arc.clone(),
        )),
        "both" => Arc::new(arazzo_exec::executor::BothEventSink::new(store_arc.clone())),
        _ => {
            print_error(
                output.format,
                output.quiet,
                &format!("unknown event sink: {events}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let event_sink: Arc<dyn arazzo_exec::executor::EventSink> =
        if let Some(webhook_url) = &webhook.webhook_url {
            let webhook_sink = Arc::new(arazzo_exec::executor::WebhookEventSink::new(
                webhook_url.clone(),
                http_client.clone(),
                base_event_sink.clone(),
            ));
            if let Some(progress) = progress_sink {
                Arc::new(super::progress::CompositeProgressSink::new(
                    progress,
                    webhook_sink,
                ))
            } else {
                webhook_sink
            }
        } else if let Some(progress) = progress_sink {
            Arc::new(super::progress::CompositeProgressSink::new(
                progress,
                base_event_sink,
            ))
        } else {
            base_event_sink
        };

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let doc_hash = hex::encode(hasher.finalize());
    let workflow_doc_json = match serde_json::to_value(&parsed.document) {
        Ok(v) => v,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to serialize workflow document: {e}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };
    let workflow_doc = match store_arc
        .upsert_workflow_doc(arazzo_store::NewWorkflowDoc {
            doc_hash,
            format: arazzo_store::DocFormat::Yaml,
            raw: content.clone(),
            doc: workflow_doc_json,
        })
        .await
    {
        Ok(doc) => doc,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to store workflow doc: {e}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let executor = arazzo_exec::Executor::new(
        exec_config,
        store_arc.clone(),
        http_client,
        secrets_provider,
        policy_gate,
        event_sink,
    );

    let run_inputs = inputs.clone().unwrap_or(serde_json::json!({}));
    let steps: Vec<arazzo_store::NewStep> = plan
        .steps
        .iter()
        .enumerate()
        .map(|(idx, s)| arazzo_store::NewStep {
            step_id: s.step_id.clone(),
            step_index: idx as i32,
            source_name: None,
            operation_id: match &s.operation {
                arazzo_core::PlanOperationRef::OperationId { operation_id, .. } => {
                    Some(operation_id.clone())
                }
                _ => None,
            },
            depends_on: s.depends_on.clone(),
        })
        .collect();

    let edges: Vec<arazzo_store::RunStepEdge> = steps
        .iter()
        .flat_map(|s| {
            s.depends_on.iter().map(|dep| arazzo_store::RunStepEdge {
                from_step_id: dep.clone(),
                to_step_id: s.step_id.clone(),
            })
        })
        .collect();

    let actual_run_id = match store_arc
        .create_run_and_steps(
            arazzo_store::NewRun {
                workflow_doc_id: workflow_doc.id,
                workflow_id: plan.summary.workflow_id.clone(),
                created_by: None,
                idempotency_key: idempotency_key.map(String::from),
                inputs: run_inputs.clone(),
                overrides: serde_json::json!({}),
            },
            steps
                .iter()
                .map(|s| arazzo_store::NewRunStep {
                    step_id: s.step_id.clone(),
                    step_index: s.step_index,
                    source_name: s.source_name.clone(),
                    operation_id: s.operation_id.clone(),
                    depends_on: s.depends_on.clone(),
                })
                .collect(),
            edges,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to create run: {e}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let run_uuid = actual_run_id;

    let result = executor
        .execute_run(run_uuid, wf, &compiled, &run_inputs, Some(&parsed.document))
        .await;

    match result {
        Ok(exec_result) => {
            let res = ExecuteResult {
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
            if res.steps_failed > 0 {
                exit_codes::RUN_FAILED
            } else {
                exit_codes::SUCCESS
            }
        }
        Err(e) => {
            let res = ExecuteResult {
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

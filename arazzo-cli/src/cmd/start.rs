use std::path::Path;
use std::sync::Arc;

use arazzo_core::{parse_document_str, plan_document, DocumentFormat, PlanOptions};
#[allow(unused_imports)]
use arazzo_store::StateStore;
use serde::Serialize;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::{
    ConcurrencyArgs, OpenApiArgs, OutputArgs, PolicyArgs, RetryArgs, SecretsArgs, StoreArgs,
};

use super::config::{get_database_url, load_inputs, merge_set_inputs};
use crate::utils::redact_url_password;

#[derive(Serialize)]
struct StartResult {
    run_id: String,
    status: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn start_cmd(
    path: &Path,
    workflow_id: Option<&str>,
    inputs_path: Option<&Path>,
    set_inputs: &[String],
    idempotency_key: Option<&str>,
    output: OutputArgs,
    store: StoreArgs,
    _openapi: OpenApiArgs,
    _secrets: SecretsArgs,
    _policy: PolicyArgs,
    _concurrency: ConcurrencyArgs,
    _retry: RetryArgs,
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

    let database_url = match get_database_url(store.store, &output) {
        Some(u) => u,
        None => return exit_codes::RUNTIME_ERROR,
    };

    let pg = match arazzo_store::PostgresStore::connect(&database_url, 5).await {
        Ok(s) => s,
        Err(e) => {
            let safe_url = redact_url_password(&database_url);
            print_error(output.format, output.quiet, &format!("database connection failed to {}: {e}. Check your DATABASE_URL and ensure Postgres is running.", safe_url));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let store_arc: Arc<dyn arazzo_store::StateStore> = Arc::new(pg);
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let doc_hash = hex::encode(hasher.finalize());
    let workflow_doc = match store_arc
        .upsert_workflow_doc(arazzo_store::NewWorkflowDoc {
            doc_hash,
            format: arazzo_store::DocFormat::Yaml,
            raw: content.clone(),
            doc: serde_json::to_value(&parsed.document).unwrap_or_default(),
        })
        .await
    {
        Ok(d) => d,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to store workflow: {e}"),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

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

    let run_id = match store_arc
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

    let result = StartResult {
        run_id: run_id.to_string(),
        status: "queued".to_string(),
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("{}", run_id);
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}

use std::time::Duration;

use arazzo_core::types::{ArazzoDocument, Step, Workflow};
use arazzo_store::{AttemptStatus, StateStore};
use serde_json::json;
use uuid::Uuid;

use crate::executor::eval::ResponseContext;
use crate::executor::failure::{decide_failure, decide_network_failure};
use crate::executor::http::HttpClient;
use crate::executor::request::{build_request, SecretsPolicyForSource};
use crate::executor::response::{
    compute_outputs, evaluate_success, parse_body_json, request_to_json, response_to_json,
};
use crate::policy::{PolicyGate, PolicyOverrides};
use crate::retry::RetryConfig;
use crate::secrets::SecretsProvider;

#[derive(Debug)]
pub enum StepResult {
    Succeeded {
        outputs: serde_json::Value,
    },
    Retry {
        delay_ms: i64,
        error: serde_json::Value,
    },
    Failed {
        error: serde_json::Value,
        end_run: bool,
    },
}

pub struct Worker<'a> {
    pub store: &'a dyn StateStore,
    pub http: &'a dyn HttpClient,
    pub secrets: &'a dyn SecretsProvider,
    pub policy_gate: &'a PolicyGate,
    pub retry: &'a RetryConfig,
    pub event_sink: &'a dyn crate::executor::EventSink,
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_step_attempt(
    worker: &Worker<'_>,
    run_id: Uuid,
    source_name: &str,
    step_row_id: Uuid,
    step: &Step,
    _workflow: &Workflow,
    resolved_op: &crate::openapi::ResolvedOperation,
    inputs: &serde_json::Value,
    document: Option<&ArazzoDocument>,
) -> StepResult {
    let eff_policy = worker
        .policy_gate
        .effective_for_source(source_name, &PolicyOverrides::default());
    let secrets_policy = SecretsPolicyForSource {
        allow_secrets_in_url: eff_policy.allow_secrets_in_url,
    };

    let req_result = build_request(
        worker.store,
        worker.secrets,
        &secrets_policy,
        run_id,
        step,
        resolved_op,
        inputs,
        document,
    )
    .await;

    let (req_parts, secret_derived_headers, body_contains_secrets) = match req_result {
        Ok(r) => (r.parts, r.secret_derived_headers, r.body_contains_secrets),
        Err(e) => {
            return StepResult::Failed {
                error: json!({"type":"build","message":e}),
                end_run: true,
            }
        }
    };

    let request_sanitized = match worker.policy_gate.apply_request(
        source_name,
        &req_parts,
        &secret_derived_headers,
        body_contains_secrets,
    ) {
        Ok(s) => s,
        Err(e) => {
            return StepResult::Failed {
                error: json!({"type":"policy","message":e.to_string()}),
                end_run: true,
            }
        }
    };

    let request_json = request_to_json(&request_sanitized);
    let (attempt_id, attempt_no) = match worker
        .store
        .insert_attempt_auto(step_row_id, request_json.clone())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return StepResult::Failed {
                error: json!({"type":"store","message":e.to_string()}),
                end_run: true,
            }
        }
    };

    worker
        .event_sink
        .emit(crate::executor::Event::AttemptStarted {
            run_id,
            step_id: step.step_id.clone(),
            attempt_no,
        })
        .await;

    let timeout = Duration::from_secs(30);
    let max_response_bytes = 4 * 1024 * 1024;

    let sent = worker
        .http
        .send(req_parts, timeout, max_response_bytes)
        .await;

    match sent {
        Ok(resp) => {
            let resp_sanitized =
                match worker
                    .policy_gate
                    .apply_response(source_name, &resp, &secret_derived_headers)
                {
                    Ok(s) => s,
                    Err(e) => {
                        finish_attempt_failed(
                            worker.store,
                            worker.event_sink,
                            run_id,
                            &step.step_id,
                            attempt_id,
                            attempt_no,
                            &e.to_string(),
                        )
                        .await;
                        return StepResult::Failed {
                            error: json!({"type":"policy","message":e.to_string()}),
                            end_run: true,
                        };
                    }
                };

            let resp_json = response_to_json(&resp_sanitized);
            let body_json = parse_body_json(&resp);
            let resp_ctx = ResponseContext {
                status: resp.status,
                headers: &resp.headers,
                body: &resp.body,
                body_json,
            };

            if evaluate_success(step, &resp_ctx) {
                let outputs = compute_outputs(worker.store, run_id, inputs, step, &resp_ctx).await;
                let _ = worker
                    .store
                    .finish_attempt(
                        attempt_id,
                        AttemptStatus::Succeeded,
                        resp_json,
                        None,
                        None,
                        None,
                    )
                    .await;
                StepResult::Succeeded { outputs }
            } else {
                let _ = worker
                    .store
                    .finish_attempt(
                        attempt_id,
                        AttemptStatus::Failed,
                        resp_json,
                        Some(json!({"type":"http","status":resp.status})),
                        None,
                        None,
                    )
                    .await;
                decide_failure(worker.retry, step, attempt_no as usize, &resp)
            }
        }
        Err(err) => {
            let _ = worker
                .store
                .finish_attempt(
                    attempt_id,
                    AttemptStatus::Failed,
                    json!({}),
                    Some(json!({"type":"network","message":err.to_string()})),
                    None,
                    None,
                )
                .await;
            worker
                .event_sink
                .emit(crate::executor::Event::AttemptFinished {
                    run_id,
                    step_id: step.step_id.clone(),
                    attempt_no,
                    succeeded: false,
                })
                .await;
            decide_network_failure(worker.retry, step, attempt_no as usize, &err)
        }
    }
}

async fn finish_attempt_failed(
    store: &dyn StateStore,
    event_sink: &dyn crate::executor::EventSink,
    run_id: Uuid,
    step_id: &str,
    attempt_id: Uuid,
    attempt_no: i32,
    msg: &str,
) {
    let _ = store
        .finish_attempt(
            attempt_id,
            AttemptStatus::Failed,
            json!({}),
            Some(json!({"type":"policy","message":msg})),
            None,
            None,
        )
        .await;
    event_sink
        .emit(crate::executor::Event::AttemptFinished {
            run_id,
            step_id: step_id.to_string(),
            attempt_no,
            succeeded: false,
        })
        .await;
}

use std::sync::Arc;

use arazzo_core::types::{ArazzoDocument, Step, Workflow};
use arazzo_store::{RunStatus, StateStore};
use uuid::Uuid;

use crate::executor::concurrency::ConcurrencyPermit;
use crate::executor::events::{Event, EventSink};
use crate::executor::http::HttpClient;
use crate::executor::worker::{execute_step_attempt, StepResult, Worker};
use crate::openapi::ResolvedOperation;
use crate::policy::PolicyGate;
use crate::retry::RetryConfig;
use crate::secrets::SecretsProvider;

pub struct StepContext {
    pub run_id: Uuid,
    pub step_row_id: Uuid,
    pub step_id: String,
    pub source_name: Option<String>,
    pub step: Step,
    pub workflow: Workflow,
    pub resolved_op: ResolvedOperation,
    pub inputs: serde_json::Value,
    pub document: Option<ArazzoDocument>,
}

pub struct StepDeps {
    pub store: Arc<dyn StateStore>,
    pub http: Arc<dyn HttpClient>,
    pub secrets: Arc<dyn SecretsProvider>,
    pub policy_gate: Arc<PolicyGate>,
    pub retry: RetryConfig,
    pub event_sink: Arc<dyn EventSink>,
}

pub async fn run_step(ctx: StepContext, deps: StepDeps, _permit: ConcurrencyPermit) -> StepResult {
    deps.event_sink
        .emit(Event::StepStarted {
            run_id: ctx.run_id,
            step_id: ctx.step_id.clone(),
        })
        .await;

    let worker = Worker {
        store: deps.store.as_ref(),
        http: deps.http.as_ref(),
        secrets: deps.secrets.as_ref(),
        policy_gate: deps.policy_gate.as_ref(),
        retry: &deps.retry,
        event_sink: deps.event_sink.as_ref(),
    };

    let result = execute_step_attempt(
        &worker,
        ctx.run_id,
        ctx.source_name.as_deref().unwrap_or(""),
        ctx.step_row_id,
        &ctx.step,
        &ctx.workflow,
        &ctx.resolved_op,
        &ctx.inputs,
        ctx.document.as_ref(),
    )
    .await;

    apply_result(&deps, ctx.run_id, &ctx.step_id, &result).await;
    result
}

async fn apply_result(deps: &StepDeps, run_id: Uuid, step_id: &str, result: &StepResult) {
    match result {
        StepResult::Succeeded { outputs } => {
            deps.store
                .mark_step_succeeded(run_id, step_id, outputs.clone())
                .await
                .ok();
            deps.event_sink
                .emit(Event::StepSucceeded {
                    run_id,
                    step_id: step_id.to_string(),
                })
                .await;
        }
        StepResult::Retry { delay_ms, error } => {
            deps.store
                .schedule_retry(run_id, step_id, *delay_ms, error.clone())
                .await
                .ok();
            deps.event_sink
                .emit(Event::StepRetryScheduled {
                    run_id,
                    step_id: step_id.to_string(),
                    delay_ms: *delay_ms,
                })
                .await;
        }
        StepResult::Failed { error, end_run } => {
            deps.store
                .mark_step_failed(run_id, step_id, error.clone())
                .await
                .ok();
            deps.event_sink
                .emit(Event::StepFailed {
                    run_id,
                    step_id: step_id.to_string(),
                })
                .await;
            if *end_run {
                deps.store
                    .mark_run_finished(run_id, RunStatus::Failed, Some(error.clone()))
                    .await
                    .ok();
            }
        }
    }
}

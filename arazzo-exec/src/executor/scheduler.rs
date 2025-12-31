use std::sync::Arc;

use arazzo_core::types::{ArazzoDocument, Workflow};
use arazzo_store::{RunStatus, StateStore};
use uuid::Uuid;

use crate::compile::CompiledPlan;
use crate::executor::concurrency::ConcurrencyLimits;
use crate::executor::events::{Event, EventSink};
use crate::executor::http::HttpClient;
use crate::executor::result::{ExecutionError, ExecutionResult};
use crate::executor::step_runner::{run_step, StepContext, StepDeps};
use crate::executor::types::ExecutorConfig;
use crate::executor::worker::StepResult;
use crate::policy::PolicyGate;
use crate::secrets::SecretsProvider;

pub struct Executor {
    config: ExecutorConfig,
    store: Arc<dyn StateStore>,
    http: Arc<dyn HttpClient>,
    secrets: Arc<dyn SecretsProvider>,
    policy_gate: Arc<PolicyGate>,
    event_sink: Arc<dyn EventSink>,
}

impl Executor {
    pub fn new(
        config: ExecutorConfig,
        store: Arc<dyn StateStore>,
        http: Arc<dyn HttpClient>,
        secrets: Arc<dyn SecretsProvider>,
        policy_gate: Arc<PolicyGate>,
        event_sink: Arc<dyn EventSink>,
    ) -> Self {
        Self { config, store, http, secrets, policy_gate, event_sink }
    }

    pub async fn execute_run(
        &self,
        run_id: Uuid,
        workflow: &Workflow,
        compiled: &CompiledPlan,
        inputs: &serde_json::Value,
        document: Option<&ArazzoDocument>,
    ) -> Result<ExecutionResult, ExecutionError> {
        let limits = ConcurrencyLimits::new(
            self.config.global_concurrency,
            &self.config.per_source_concurrency,
        );

        self.emit_run_started(run_id, workflow).await;
        let _ = self.store.mark_run_started(run_id).await;

        let mut result = ExecutionResult::default();
        loop {
            let claimed = self.claim_steps(run_id).await?;
            if claimed.is_empty() {
                if self.is_run_complete(run_id).await? {
                    self.emit_run_finished(run_id, RunStatus::Succeeded).await;
                    break;
                }
                tokio::time::sleep(self.config.poll_interval).await;
                continue;
            }

            let handles = self.spawn_steps(run_id, &claimed, workflow, compiled, inputs, &limits, document).await?;
            self.collect_results(handles, &mut result).await?;
        }

        Ok(result)
    }

    async fn emit_run_started(&self, run_id: Uuid, workflow: &Workflow) {
        self.event_sink
            .emit(Event::RunStarted { run_id, workflow_id: workflow.workflow_id.clone() })
            .await;
    }

    async fn emit_run_finished(&self, run_id: Uuid, status: RunStatus) {
        self.event_sink
            .emit(Event::RunFinished { run_id, status })
            .await;
    }

    async fn claim_steps(&self, run_id: Uuid) -> Result<Vec<arazzo_store::RunStep>, ExecutionError> {
        self.store
            .claim_runnable_steps(run_id, self.config.global_concurrency as i64)
            .await
            .map_err(ExecutionError::Store)
    }

    async fn is_run_complete(&self, run_id: Uuid) -> Result<bool, ExecutionError> {
        let runnable = self.store.claim_runnable_steps(run_id, 1).await.map_err(ExecutionError::Store)?;
        if !runnable.is_empty() {
            return Ok(false);
        }
        
        let all_steps = self.store.get_run_steps(run_id).await.map_err(ExecutionError::Store)?;
        if all_steps.is_empty() {
            return Ok(false);
        }
        
        let all_terminal = all_steps.iter().all(|s| {
            matches!(s.status.as_str(), "succeeded" | "failed" | "skipped")
        });
        
        if all_terminal {
            if let Ok(Some(run)) = self.store.get_run(run_id).await {
                if matches!(run.status.as_str(), "pending" | "queued" | "running") {
                    let _ = self.store.mark_run_finished(run_id, RunStatus::Succeeded, None).await;
                }
            }
            return Ok(true);
        }
        
        Ok(false)
    }

    async fn spawn_steps(
        &self,
        run_id: Uuid,
        claimed: &[arazzo_store::RunStep],
        workflow: &Workflow,
        compiled: &CompiledPlan,
        inputs: &serde_json::Value,
        limits: &ConcurrencyLimits,
        document: Option<&ArazzoDocument>,
    ) -> Result<Vec<(String, tokio::task::JoinHandle<StepResult>)>, ExecutionError> {
        let mut handles = Vec::new();

        for step_row in claimed {
            let step_id = step_row.step_id.clone();

            let step = workflow
                .steps
                .iter()
                .find(|s| s.step_id == step_id)
                .ok_or_else(|| ExecutionError::StepNotFound(step_id.clone()))?;

            let compiled_step = compiled
                .steps
                .iter()
                .find(|s| s.step_id == step_id)
                .ok_or_else(|| ExecutionError::CompiledStepNotFound(step_id.clone()))?;

            let resolved_op = compiled_step
                .operation
                .as_ref()
                .ok_or_else(|| ExecutionError::MissingOperation(step_id.clone()))?;

            let permit = limits.acquire(step_row.source_name.as_deref()).await;

            let ctx = StepContext {
                run_id,
                step_row_id: step_row.id,
                step_id: step_id.clone(),
                source_name: step_row.source_name.clone(),
                step: step.clone(),
                workflow: workflow.clone(),
                resolved_op: resolved_op.clone(),
                inputs: inputs.clone(),
                document: document.cloned(),
            };

            let deps = StepDeps {
                store: self.store.clone(),
                http: self.http.clone(),
                secrets: self.secrets.clone(),
                policy_gate: self.policy_gate.clone(),
                retry: self.config.retry.clone(),
                event_sink: self.event_sink.clone(),
            };

            let handle = tokio::spawn(async move { run_step(ctx, deps, permit).await });
            handles.push((step_id, handle));
        }

        Ok(handles)
    }

    async fn collect_results(
        &self,
        handles: Vec<(String, tokio::task::JoinHandle<StepResult>)>,
        result: &mut ExecutionResult,
    ) -> Result<(), ExecutionError> {
        for (step_id, handle) in handles {
            match handle.await {
                Ok(StepResult::Succeeded { .. }) => result.record_success(),
                Ok(StepResult::Retry { .. }) => result.record_retry(),
                Ok(StepResult::Failed { .. }) => result.record_failure(),
                Err(e) => return Err(ExecutionError::TaskJoin(format!("step {}: {}", step_id, e))),
            }
        }
        Ok(())
    }
}

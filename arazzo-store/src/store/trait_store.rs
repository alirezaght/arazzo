use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::store::types::*;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn upsert_workflow_doc(&self, doc: NewWorkflowDoc) -> Result<WorkflowDoc, StoreError>;

    async fn get_workflow_doc(&self, id: Uuid) -> Result<Option<WorkflowDoc>, StoreError>;

    async fn create_run_and_steps(
        &self,
        run: NewRun,
        steps: Vec<NewRunStep>,
        edges: Vec<RunStepEdge>,
    ) -> Result<Uuid, StoreError>;

    async fn claim_runnable_steps(
        &self,
        run_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RunStep>, StoreError>;

    /// Insert a new attempt with an automatically computed `attempt_no` (append-only).
    async fn insert_attempt_auto(
        &self,
        run_step_id: Uuid,
        request: JsonValue,
    ) -> Result<(Uuid, i32), StoreError>;

    async fn finish_attempt(
        &self,
        attempt_id: Uuid,
        status: AttemptStatus,
        response: JsonValue,
        error: Option<JsonValue>,
        duration_ms: Option<i32>,
        finished_at: Option<DateTime<Utc>>,
    ) -> Result<(), StoreError>;

    async fn mark_step_succeeded(
        &self,
        run_id: Uuid,
        step_id: &str,
        outputs: JsonValue,
    ) -> Result<(), StoreError>;

    /// Read outputs for an already-succeeded step (used for evaluating dependent expressions).
    async fn get_step_outputs(&self, run_id: Uuid, step_id: &str) -> Result<JsonValue, StoreError>;

    async fn schedule_retry(
        &self,
        run_id: Uuid,
        step_id: &str,
        delay_ms: i64,
        error: JsonValue,
    ) -> Result<(), StoreError>;

    async fn mark_step_failed(
        &self,
        run_id: Uuid,
        step_id: &str,
        error: JsonValue,
    ) -> Result<(), StoreError>;

    async fn mark_run_started(&self, run_id: Uuid) -> Result<(), StoreError>;

    async fn mark_run_finished(
        &self,
        run_id: Uuid,
        status: RunStatus,
        error: Option<JsonValue>,
    ) -> Result<(), StoreError>;

    async fn append_event(&self, event: NewEvent) -> Result<(), StoreError>;

    async fn get_run(&self, run_id: Uuid) -> Result<Option<WorkflowRun>, StoreError>;

    async fn get_run_steps(&self, run_id: Uuid) -> Result<Vec<RunStep>, StoreError>;

    /// Reset steps stuck in 'running' state (after crash). Returns count of reset steps.
    async fn reset_stale_running_steps(&self, run_id: Uuid) -> Result<i64, StoreError>;

    async fn get_step_attempts(&self, run_step_id: Uuid) -> Result<Vec<StepAttempt>, StoreError>;

    async fn get_events_after(
        &self,
        run_id: Uuid,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<RunEvent>, StoreError>;

    async fn check_run_status(&self, run_id: Uuid) -> Result<String, StoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store error: {0}")]
    Other(String),
}

impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        StoreError::Other(e.to_string())
    }
}

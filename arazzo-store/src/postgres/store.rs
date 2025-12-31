use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::store::{
    AttemptStatus, NewEvent, NewRun, NewRunStep, NewStep, NewWorkflowDoc, RunEvent, RunStatus,
    RunStep, RunStepEdge, StateStore, StepAttempt, StoreError, WorkflowDoc, WorkflowRun,
};

use super::events;
use super::runs;
use super::steps;

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, StoreError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_run_and_steps(
        &self,
        run_id: Uuid,
        workflow_doc_id: Uuid,
        workflow_id: &str,
        created_by: Option<String>,
        idempotency_key: Option<String>,
        inputs: &JsonValue,
        overrides: &JsonValue,
        steps: &[NewStep],
    ) -> Result<Uuid, StoreError> {
        runs::create_run_with_id(
            &self.pool, run_id, workflow_doc_id, workflow_id,
            created_by, idempotency_key, inputs, overrides, steps,
        ).await
    }

    pub async fn mark_run_started(&self, run_id: Uuid) -> Result<(), StoreError> {
        runs::mark_run_started(&self.pool, run_id).await
    }

    pub async fn mark_run_finished(
        &self,
        run_id: Uuid,
        status: &str,
        error: Option<JsonValue>,
    ) -> Result<(), StoreError> {
        runs::mark_run_finished_str(&self.pool, run_id, status, error).await
    }
}

#[async_trait::async_trait]
impl StateStore for PostgresStore {
    async fn upsert_workflow_doc(&self, doc: NewWorkflowDoc) -> Result<WorkflowDoc, StoreError> {
        events::upsert_workflow_doc(&self.pool, doc).await
    }

    async fn get_workflow_doc(&self, id: Uuid) -> Result<Option<WorkflowDoc>, StoreError> {
        events::get_workflow_doc(&self.pool, id).await
    }

    async fn create_run_and_steps(
        &self,
        run: NewRun,
        steps: Vec<NewRunStep>,
        edges: Vec<RunStepEdge>,
    ) -> Result<Uuid, StoreError> {
        runs::create_run(&self.pool, run, steps, edges).await
    }

    async fn claim_runnable_steps(&self, run_id: Uuid, limit: i64) -> Result<Vec<RunStep>, StoreError> {
        steps::claim_runnable_steps(&self.pool, run_id, limit).await
    }

    async fn insert_attempt_auto(&self, run_step_id: Uuid, request: JsonValue) -> Result<(Uuid, i32), StoreError> {
        steps::insert_attempt_auto(&self.pool, run_step_id, request).await
    }

    async fn finish_attempt(
        &self,
        attempt_id: Uuid,
        status: AttemptStatus,
        response: JsonValue,
        error: Option<JsonValue>,
        duration_ms: Option<i32>,
        finished_at: Option<DateTime<Utc>>,
    ) -> Result<(), StoreError> {
        steps::finish_attempt(&self.pool, attempt_id, status, response, error, duration_ms, finished_at).await
    }

    async fn mark_step_succeeded(&self, run_id: Uuid, step_id: &str, outputs: JsonValue) -> Result<(), StoreError> {
        steps::mark_step_succeeded(&self.pool, run_id, step_id, outputs).await
    }

    async fn get_step_outputs(&self, run_id: Uuid, step_id: &str) -> Result<JsonValue, StoreError> {
        steps::get_step_outputs(&self.pool, run_id, step_id).await
    }

    async fn schedule_retry(&self, run_id: Uuid, step_id: &str, delay_ms: i64, error: JsonValue) -> Result<(), StoreError> {
        steps::schedule_retry(&self.pool, run_id, step_id, delay_ms, error).await
    }

    async fn mark_step_failed(&self, run_id: Uuid, step_id: &str, error: JsonValue) -> Result<(), StoreError> {
        steps::mark_step_failed(&self.pool, run_id, step_id, error).await
    }

    async fn mark_run_started(&self, run_id: Uuid) -> Result<(), StoreError> {
        runs::mark_run_started(&self.pool, run_id).await
    }

    async fn mark_run_finished(&self, run_id: Uuid, status: RunStatus, error: Option<JsonValue>) -> Result<(), StoreError> {
        runs::mark_run_finished_enum(&self.pool, run_id, status, error).await
    }

    async fn append_event(&self, event: NewEvent) -> Result<(), StoreError> {
        events::append_event(&self.pool, event).await
    }

    async fn get_run(&self, run_id: Uuid) -> Result<Option<WorkflowRun>, StoreError> {
        runs::get_run(&self.pool, run_id).await
    }

    async fn get_run_steps(&self, run_id: Uuid) -> Result<Vec<RunStep>, StoreError> {
        steps::get_run_steps(&self.pool, run_id).await
    }

    async fn reset_stale_running_steps(&self, run_id: Uuid) -> Result<i64, StoreError> {
        steps::reset_stale_running_steps(&self.pool, run_id).await
    }

    async fn get_step_attempts(&self, run_step_id: Uuid) -> Result<Vec<StepAttempt>, StoreError> {
        steps::get_step_attempts(&self.pool, run_step_id).await
    }

    async fn get_events_after(&self, run_id: Uuid, after_id: i64, limit: i64) -> Result<Vec<RunEvent>, StoreError> {
        events::get_events_after(&self.pool, run_id, after_id, limit).await
    }

    async fn check_run_status(&self, run_id: Uuid) -> Result<String, StoreError> {
        runs::check_run_status(&self.pool, run_id).await
    }
}

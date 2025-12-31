use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Yaml,
    Json,
}

impl DocFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocFormat::Yaml => "yaml",
            DocFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewWorkflowDoc {
    pub doc_hash: String,
    pub format: DocFormat,
    pub raw: String,
    pub doc: JsonValue,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkflowDoc {
    pub id: Uuid,
    pub doc_hash: String,
    pub format: String,
    pub raw: String,
    pub doc: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Succeeded => "succeeded",
            RunStatus::Failed => "failed",
            RunStatus::Canceled => "canceled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRun {
    pub workflow_doc_id: Uuid,
    pub workflow_id: String,
    pub created_by: Option<String>,
    pub idempotency_key: Option<String>,
    pub inputs: JsonValue,
    pub overrides: JsonValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl RunStepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStepStatus::Pending => "pending",
            RunStepStatus::Running => "running",
            RunStepStatus::Succeeded => "succeeded",
            RunStepStatus::Failed => "failed",
            RunStepStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRunStep {
    pub step_id: String,
    pub step_index: i32,
    pub source_name: Option<String>,
    pub operation_id: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunStep {
    pub id: Uuid,
    pub run_id: Uuid,
    pub step_id: String,
    pub step_index: i32,
    pub status: String,
    pub source_name: Option<String>,
    pub operation_id: Option<String>,
    pub depends_on: Vec<String>,
    pub deps_remaining: i32,
    pub next_run_at: Option<DateTime<Utc>>,
    pub outputs: JsonValue,
    pub error: Option<JsonValue>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptStatus {
    Running,
    Succeeded,
    Failed,
}

impl AttemptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttemptStatus::Running => "running",
            AttemptStatus::Succeeded => "succeeded",
            AttemptStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewAttempt {
    pub run_step_id: Uuid,
    pub attempt_no: i32,
    pub request: JsonValue,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StepAttempt {
    pub id: Uuid,
    pub run_step_id: Uuid,
    pub attempt_no: i32,
    pub status: String,
    pub request: JsonValue,
    pub response: JsonValue,
    pub error: Option<JsonValue>,
    pub duration_ms: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub run_id: Uuid,
    pub run_step_id: Option<Uuid>,
    pub r#type: String,
    pub payload: JsonValue,
}

#[derive(Debug, Clone)]
pub struct CreatedRun {
    pub run_id: Uuid,
    pub workflow_doc_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RunStepEdge {
    pub from_step_id: String,
    pub to_step_id: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkflowRun {
    pub id: Uuid,
    pub workflow_doc_id: Uuid,
    pub workflow_id: String,
    pub status: String,
    pub created_by: Option<String>,
    pub idempotency_key: Option<String>,
    pub inputs: JsonValue,
    pub overrides: JsonValue,
    pub error: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunEvent {
    pub id: i64,
    pub run_id: Uuid,
    pub run_step_id: Option<Uuid>,
    pub ts: DateTime<Utc>,
    pub event_type: String,
    pub payload: JsonValue,
}

#[derive(Debug, Clone)]
pub struct NewStep {
    pub step_id: String,
    pub step_index: i32,
    pub source_name: Option<String>,
    pub operation_id: Option<String>,
    pub depends_on: Vec<String>,
}


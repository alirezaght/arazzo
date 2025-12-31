use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use arazzo_store::{RunStatus, StateStore};

#[derive(Debug, Clone)]
pub enum Event {
    RunStarted {
        run_id: Uuid,
        workflow_id: String,
    },
    RunFinished {
        run_id: Uuid,
        status: RunStatus,
    },
    StepStarted {
        run_id: Uuid,
        step_id: String,
    },
    StepSucceeded {
        run_id: Uuid,
        step_id: String,
    },
    StepFailed {
        run_id: Uuid,
        step_id: String,
    },
    StepRetryScheduled {
        run_id: Uuid,
        step_id: String,
        delay_ms: i64,
    },
    AttemptStarted {
        run_id: Uuid,
        step_id: String,
        attempt_no: i32,
    },
    AttemptFinished {
        run_id: Uuid,
        step_id: String,
        attempt_no: i32,
        succeeded: bool,
    },
    PolicyDenied {
        run_id: Uuid,
        step_id: String,
        reason: String,
    },
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn emit(&self, event: Event);
}

pub struct CompositeEventSink {
    sinks: Vec<Box<dyn EventSink>>,
}

impl Default for CompositeEventSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeEventSink {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn add(&mut self, sink: Box<dyn EventSink>) {
        self.sinks.push(sink);
    }
}

#[async_trait]
impl EventSink for CompositeEventSink {
    async fn emit(&self, event: Event) {
        for sink in &self.sinks {
            let event_clone = event.clone();
            sink.emit(event_clone).await;
        }
    }
}

pub struct StoreEventSink {
    store: std::sync::Arc<dyn StateStore>,
}

impl StoreEventSink {
    pub fn new(store: std::sync::Arc<dyn StateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl EventSink for StoreEventSink {
    async fn emit(&self, event: Event) {
        let (run_id, step_id, event_type, payload) = match event {
            Event::RunStarted { run_id, workflow_id } => {
                (run_id, None, "run.started", json!({ "workflow_id": workflow_id }))
            }
            Event::RunFinished { run_id, status } => {
                (
                    run_id,
                    None,
                    "run.finished",
                    json!({ "status": status.as_str() }),
                )
            }
            Event::StepStarted { run_id, step_id } => {
                (
                    run_id,
                    None,
                    "step.started",
                    json!({ "step_id": step_id }),
                )
            }
            Event::StepSucceeded { run_id, step_id } => {
                (
                    run_id,
                    None,
                    "step.succeeded",
                    json!({ "step_id": step_id }),
                )
            }
            Event::StepFailed { run_id, step_id } => {
                (
                    run_id,
                    None,
                    "step.failed",
                    json!({ "step_id": step_id }),
                )
            }
            Event::StepRetryScheduled {
                run_id,
                step_id,
                delay_ms,
            } => {
                (
                    run_id,
                    None,
                    "step.retry_scheduled",
                    json!({ "step_id": step_id, "delay_ms": delay_ms }),
                )
            }
            Event::AttemptStarted {
                run_id,
                step_id,
                attempt_no,
            } => {
                (
                    run_id,
                    None,
                    "attempt.started",
                    json!({ "step_id": step_id, "attempt_no": attempt_no }),
                )
            }
            Event::AttemptFinished {
                run_id,
                step_id,
                attempt_no,
                succeeded,
            } => {
                (
                    run_id,
                    None,
                    "attempt.finished",
                    json!({
                        "step_id": step_id,
                        "attempt_no": attempt_no,
                        "succeeded": succeeded
                    }),
                )
            }
            Event::PolicyDenied {
                run_id,
                step_id,
                reason,
            } => {
                (
                    run_id,
                    None,
                    "policy.denied",
                    json!({ "step_id": step_id, "reason": reason }),
                )
            }
        };

        let _ = self
            .store
            .append_event(arazzo_store::NewEvent {
                run_id,
                run_step_id: step_id,
                r#type: event_type.to_string(),
                payload,
            })
            .await;
    }
}

pub struct StdoutEventSink;

#[async_trait]
impl EventSink for StdoutEventSink {
    async fn emit(&self, event: Event) {
        let json = match event {
            Event::RunStarted { run_id, workflow_id } => {
                json!({ "type": "run.started", "run_id": run_id.to_string(), "workflow_id": workflow_id })
            }
            Event::RunFinished { run_id, status } => {
                json!({ "type": "run.finished", "run_id": run_id.to_string(), "status": status.as_str() })
            }
            Event::StepStarted { run_id, step_id } => {
                json!({ "type": "step.started", "run_id": run_id.to_string(), "step_id": step_id })
            }
            Event::StepSucceeded { run_id, step_id } => {
                json!({ "type": "step.succeeded", "run_id": run_id.to_string(), "step_id": step_id })
            }
            Event::StepFailed { run_id, step_id } => {
                json!({ "type": "step.failed", "run_id": run_id.to_string(), "step_id": step_id })
            }
            Event::StepRetryScheduled { run_id, step_id, delay_ms } => {
                json!({ "type": "step.retry_scheduled", "run_id": run_id.to_string(), "step_id": step_id, "delay_ms": delay_ms })
            }
            Event::AttemptStarted { run_id, step_id, attempt_no } => {
                json!({ "type": "attempt.started", "run_id": run_id.to_string(), "step_id": step_id, "attempt_no": attempt_no })
            }
            Event::AttemptFinished { run_id, step_id, attempt_no, succeeded } => {
                json!({ "type": "attempt.finished", "run_id": run_id.to_string(), "step_id": step_id, "attempt_no": attempt_no, "succeeded": succeeded })
            }
            Event::PolicyDenied { run_id, step_id, reason } => {
                json!({ "type": "policy.denied", "run_id": run_id.to_string(), "step_id": step_id, "reason": reason })
            }
        };
        println!("{}", serde_json::to_string(&json).unwrap_or_default());
    }
}

pub struct BothEventSink {
    stdout: StdoutEventSink,
    store: StoreEventSink,
}

impl BothEventSink {
    pub fn new(store: std::sync::Arc<dyn StateStore>) -> Self {
        Self {
            stdout: StdoutEventSink,
            store: StoreEventSink::new(store),
        }
    }
}

#[async_trait]
impl EventSink for BothEventSink {
    async fn emit(&self, event: Event) {
        let event_clone = event.clone();
        self.stdout.emit(event_clone).await;
        self.store.emit(event).await;
    }
}

pub struct NoOpEventSink;

#[async_trait]
impl EventSink for NoOpEventSink {
    async fn emit(&self, _event: Event) {
    }
}


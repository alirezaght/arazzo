use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use arazzo_exec::executor::events::{CompositeEventSink, Event, EventSink, StoreEventSink};
use arazzo_store::{RunStatus, StateStore};
use async_trait::async_trait;

struct MockStore {
    events: Arc<tokio::sync::Mutex<Vec<String>>>,
}

#[async_trait]
impl StateStore for MockStore {
    async fn append_event(
        &self,
        event: arazzo_store::NewEvent,
    ) -> Result<(), arazzo_store::StoreError> {
        self.events.lock().await.push(event.r#type);
        Ok(())
    }

    async fn get_step_outputs(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
    ) -> Result<serde_json::Value, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn upsert_workflow_doc(
        &self,
        _doc: arazzo_store::NewWorkflowDoc,
    ) -> Result<arazzo_store::WorkflowDoc, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_workflow_doc(
        &self,
        _workflow_doc_id: uuid::Uuid,
    ) -> Result<Option<arazzo_store::WorkflowDoc>, arazzo_store::StoreError> {
        Ok(None)
    }

    async fn create_run_and_steps(
        &self,
        _run: arazzo_store::NewRun,
        _steps: Vec<arazzo_store::NewRunStep>,
        _edges: Vec<arazzo_store::RunStepEdge>,
    ) -> Result<uuid::Uuid, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn claim_runnable_steps(
        &self,
        _run_id: uuid::Uuid,
        _limit: i64,
    ) -> Result<Vec<arazzo_store::RunStep>, arazzo_store::StoreError> {
        unimplemented!()
    }


    async fn mark_step_succeeded(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _outputs: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn mark_step_failed(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _error: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn schedule_retry(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _delay_ms: i64,
        _error: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn mark_run_finished(
        &self,
        _run_id: uuid::Uuid,
        _status: arazzo_store::RunStatus,
        _error: Option<serde_json::Value>,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn mark_run_started(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn insert_attempt_auto(
        &self,
        _run_step_id: uuid::Uuid,
        _request: serde_json::Value,
    ) -> Result<(uuid::Uuid, i32), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn finish_attempt(
        &self,
        _attempt_id: uuid::Uuid,
        _status: arazzo_store::AttemptStatus,
        _response: serde_json::Value,
        _error: Option<serde_json::Value>,
        _duration_ms: Option<i32>,
        _finished_at: Option<DateTime<Utc>>,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_run(&self, _run_id: uuid::Uuid) -> Result<Option<arazzo_store::WorkflowRun>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_run_steps(&self, _run_id: uuid::Uuid) -> Result<Vec<arazzo_store::RunStep>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn reset_stale_running_steps(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<i64, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_step_attempts(&self, _run_step_id: uuid::Uuid) -> Result<Vec<arazzo_store::StepAttempt>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_events_after(&self, _run_id: uuid::Uuid, _after_id: i64, _limit: i64) -> Result<Vec<arazzo_store::RunEvent>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn check_run_status(&self, _run_id: uuid::Uuid) -> Result<String, arazzo_store::StoreError> {
        unimplemented!()
    }

}

#[tokio::test]
async fn store_event_sink_emits_run_started() {
    let store = Arc::new(MockStore {
        events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    });
    let sink = StoreEventSink::new(store.clone());

    sink.emit(Event::RunStarted {
        run_id: Uuid::new_v4(),
        workflow_id: "test".to_string(),
    })
    .await;

    let events = store.events.lock().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "run.started");
}

#[tokio::test]
async fn store_event_sink_emits_run_finished() {
    let store = Arc::new(MockStore {
        events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    });
    let sink = StoreEventSink::new(store.clone());

    sink.emit(Event::RunFinished {
        run_id: Uuid::new_v4(),
        status: RunStatus::Succeeded,
    })
    .await;

    let events = store.events.lock().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "run.finished");
}

#[tokio::test]
async fn store_event_sink_emits_step_events() {
    let store = Arc::new(MockStore {
        events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    });
    let sink = StoreEventSink::new(store.clone());
    let run_id = Uuid::new_v4();

    sink.emit(Event::StepStarted {
        run_id,
        step_id: "step1".to_string(),
    })
    .await;

    sink.emit(Event::StepSucceeded {
        run_id,
        step_id: "step1".to_string(),
    })
    .await;

    sink.emit(Event::StepFailed {
        run_id,
        step_id: "step2".to_string(),
    })
    .await;

    let events = store.events.lock().await;
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], "step.started");
    assert_eq!(events[1], "step.succeeded");
    assert_eq!(events[2], "step.failed");
}

#[tokio::test]
async fn composite_event_sink_forwards_to_all_sinks() {
    let store1 = Arc::new(MockStore {
        events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    });
    let store2 = Arc::new(MockStore {
        events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    });

    let mut composite = CompositeEventSink::new();
    composite.add(Box::new(StoreEventSink::new(store1.clone())));
    composite.add(Box::new(StoreEventSink::new(store2.clone())));

    composite
        .emit(Event::RunStarted {
            run_id: Uuid::new_v4(),
            workflow_id: "test".to_string(),
        })
        .await;

    let events1 = store1.events.lock().await;
    let events2 = store2.events.lock().await;
    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 1);
}


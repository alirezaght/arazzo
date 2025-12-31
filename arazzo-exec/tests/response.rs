use std::collections::BTreeMap;

use arazzo_core::types::Step;
use arazzo_exec::executor::eval::ResponseContext;
use arazzo_exec::executor::response::{
    compute_outputs, evaluate_success, parse_body_json, request_to_json, response_to_json,
};
use arazzo_exec::policy::sanitize::{SanitizedBody, SanitizedHeaders};
use arazzo_exec::policy::{HttpResponseParts, RequestGateResult, ResponseGateResult};
use arazzo_store::StateStore;
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

struct MockStore;

#[async_trait]
impl StateStore for MockStore {
    async fn get_step_outputs(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
    ) -> Result<serde_json::Value, arazzo_store::StoreError> {
        Ok(json!({}))
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
        _finished_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), arazzo_store::StoreError> {
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

    async fn schedule_retry(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _delay_ms: i64,
        _error: serde_json::Value,
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

    async fn mark_run_started(&self, _run_id: uuid::Uuid) -> Result<(), arazzo_store::StoreError> {
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

    async fn append_event(
        &self,
        _event: arazzo_store::NewEvent,
    ) -> Result<(), arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_run(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<Option<arazzo_store::WorkflowRun>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_run_steps(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<Vec<arazzo_store::RunStep>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn reset_stale_running_steps(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<i64, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_step_attempts(
        &self,
        _run_step_id: uuid::Uuid,
    ) -> Result<Vec<arazzo_store::StepAttempt>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_events_after(
        &self,
        _run_id: uuid::Uuid,
        _after_id: i64,
        _limit: i64,
    ) -> Result<Vec<arazzo_store::RunEvent>, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn check_run_status(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<String, arazzo_store::StoreError> {
        unimplemented!()
    }
}

#[test]
fn parse_body_json_valid() {
    let resp = HttpResponseParts {
        status: 200,
        headers: BTreeMap::new(),
        body: b"{\"key\":\"value\"}".to_vec(),
    };
    let result = parse_body_json(&resp);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), json!({"key": "value"}));
}

#[test]
fn parse_body_json_invalid() {
    let resp = HttpResponseParts {
        status: 200,
        headers: BTreeMap::new(),
        body: b"not json".to_vec(),
    };
    let result = parse_body_json(&resp);
    assert!(result.is_none());
}

#[test]
fn evaluate_success_defaults_to_2xx() {
    let step = Step {
        step_id: "test".to_string(),
        description: None,
        operation_id: None,
        operation_path: None,
        workflow_id: None,
        parameters: None,
        request_body: None,
        success_criteria: None,
        outputs: None,
        on_success: None,
        on_failure: None,
        extensions: BTreeMap::new(),
    };

    let headers = BTreeMap::new();
    let resp_ctx = ResponseContext {
        status: 200,
        headers: &headers,
        body: b"{}",
        body_json: None,
    };
    assert!(evaluate_success(&step, &resp_ctx));

    let resp_ctx_404 = ResponseContext {
        status: 404,
        headers: &headers,
        body: b"{}",
        body_json: None,
    };
    assert!(!evaluate_success(&step, &resp_ctx_404));
}

#[tokio::test]
async fn compute_outputs_extracts_from_response() {
    let step = Step {
        step_id: "test".to_string(),
        description: None,
        operation_id: None,
        operation_path: None,
        workflow_id: None,
        parameters: None,
        request_body: None,
        success_criteria: None,
        outputs: Some({
            let mut m = BTreeMap::new();
            m.insert("userId".to_string(), "$response.body#/id".to_string());
            m.insert("status".to_string(), "$statusCode".to_string());
            m
        }),
        on_success: None,
        on_failure: None,
        extensions: BTreeMap::new(),
    };

    let headers = BTreeMap::new();
    let resp_ctx = ResponseContext {
        status: 200,
        headers: &headers,
        body: b"{\"id\":123}",
        body_json: Some(json!({"id": 123})),
    };

    let outputs = compute_outputs(&MockStore, Uuid::new_v4(), &json!({}), &step, &resp_ctx).await;
    assert_eq!(outputs["status"], json!(200));
}

#[test]
fn request_to_json_serializes() {
    let req = RequestGateResult {
        url: "https://example.com/test".to_string(),
        method: "POST".to_string(),
        headers: SanitizedHeaders {
            headers: {
                let mut m = BTreeMap::new();
                m.insert("Content-Type".to_string(), "application/json".to_string());
                m
            },
        },
        body: SanitizedBody {
            bytes: b"{\"test\":true}".to_vec(),
            truncated: false,
        },
    };

    let json = request_to_json(&req);
    assert_eq!(json["method"], "POST");
    assert_eq!(json["url"], "https://example.com/test");
    assert_eq!(json["body"], "{\"test\":true}");
    assert_eq!(json["body_truncated"], false);
}

#[test]
fn response_to_json_serializes() {
    let resp = ResponseGateResult {
        status: 200,
        headers: SanitizedHeaders {
            headers: {
                let mut m = BTreeMap::new();
                m.insert("Content-Type".to_string(), "application/json".to_string());
                m
            },
        },
        body: SanitizedBody {
            bytes: b"{\"success\":true}".to_vec(),
            truncated: false,
        },
    };

    let json = response_to_json(&resp);
    assert_eq!(json["status"], 200);
    assert_eq!(json["body"], "{\"success\":true}");
    assert_eq!(json["body_truncated"], false);
}

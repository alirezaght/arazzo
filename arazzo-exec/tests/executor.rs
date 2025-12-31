use std::collections::BTreeMap;
use std::time::Duration;

use arazzo_exec::executor::{EventSink, HttpClient, HttpError, StepResult, Worker};
use arazzo_exec::policy::{HttpRequestParts, HttpResponseParts, PolicyConfig, PolicyGate};
use arazzo_exec::retry::RetryConfig;
use arazzo_exec::secrets::{SecretValue, SecretsProvider};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

// Mock HTTP client
struct MockHttpClient {
    response: HttpResponseParts,
    fail_with: Option<HttpError>,
}

#[async_trait]
impl HttpClient for MockHttpClient {
    async fn send(
        &self,
        _req: HttpRequestParts,
        _timeout: Duration,
        _max_response_bytes: usize,
    ) -> Result<HttpResponseParts, HttpError> {
        if let Some(ref err) = self.fail_with {
            return Err(err.clone());
        }
        Ok(self.response.clone())
    }
}

// Mock event sink for tests
struct MockEventSink;

#[async_trait]
impl EventSink for MockEventSink {
    async fn emit(&self, _event: arazzo_exec::executor::Event) {
        // No-op for tests
    }
}

// Mock store that doesn't require DB
struct MockStore;

#[async_trait::async_trait]
impl arazzo_store::StateStore for MockStore {
    async fn upsert_workflow_doc(
        &self,
        _doc: arazzo_store::NewWorkflowDoc,
    ) -> Result<arazzo_store::WorkflowDoc, arazzo_store::StoreError> {
        unimplemented!()
    }

    async fn get_workflow_doc(
        &self,
        _id: uuid::Uuid,
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
        Ok(vec![])
    }

    async fn insert_attempt_auto(
        &self,
        _run_step_id: uuid::Uuid,
        _request: serde_json::Value,
    ) -> Result<(uuid::Uuid, i32), arazzo_store::StoreError> {
        Ok((uuid::Uuid::new_v4(), 1))
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
        Ok(())
    }

    async fn mark_step_succeeded(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _outputs: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn get_step_outputs(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
    ) -> Result<serde_json::Value, arazzo_store::StoreError> {
        Ok(serde_json::json!({}))
    }

    async fn schedule_retry(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _delay_ms: i64,
        _error: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn mark_step_failed(
        &self,
        _run_id: uuid::Uuid,
        _step_id: &str,
        _error: serde_json::Value,
    ) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn mark_run_started(&self, _run_id: uuid::Uuid) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn mark_run_finished(
        &self,
        _run_id: uuid::Uuid,
        _status: arazzo_store::RunStatus,
        _error: Option<serde_json::Value>,
    ) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn append_event(
        &self,
        _event: arazzo_store::NewEvent,
    ) -> Result<(), arazzo_store::StoreError> {
        Ok(())
    }

    async fn get_run(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<Option<arazzo_store::WorkflowRun>, arazzo_store::StoreError> {
        Ok(None)
    }

    async fn get_run_steps(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<Vec<arazzo_store::RunStep>, arazzo_store::StoreError> {
        Ok(vec![])
    }

    async fn reset_stale_running_steps(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<i64, arazzo_store::StoreError> {
        Ok(0)
    }

    async fn get_step_attempts(
        &self,
        _run_step_id: uuid::Uuid,
    ) -> Result<Vec<arazzo_store::StepAttempt>, arazzo_store::StoreError> {
        Ok(vec![])
    }

    async fn get_events_after(
        &self,
        _run_id: uuid::Uuid,
        _after_id: i64,
        _limit: i64,
    ) -> Result<Vec<arazzo_store::RunEvent>, arazzo_store::StoreError> {
        Ok(vec![])
    }

    async fn check_run_status(
        &self,
        _run_id: uuid::Uuid,
    ) -> Result<String, arazzo_store::StoreError> {
        Ok("succeeded".to_string())
    }
}

// Mock secrets provider
struct NoOpSecretsProvider;

#[async_trait]
impl SecretsProvider for NoOpSecretsProvider {
    async fn get(
        &self,
        ref_: &arazzo_exec::secrets::SecretRef,
    ) -> Result<SecretValue, arazzo_exec::secrets::SecretError> {
        Err(arazzo_exec::secrets::SecretError::NotFound(ref_.clone()))
    }
}

fn make_step(step_id: &str) -> arazzo_core::types::Step {
    arazzo_core::types::Step {
        step_id: step_id.to_string(),
        description: None,
        operation_id: Some("getUsers".to_string()),
        operation_path: None,
        workflow_id: None,
        parameters: None,
        request_body: None,
        success_criteria: None,
        on_success: None,
        on_failure: None,
        outputs: None,
        extensions: Default::default(),
    }
}

fn make_workflow() -> arazzo_core::types::Workflow {
    arazzo_core::types::Workflow {
        workflow_id: "test-workflow".to_string(),
        summary: None,
        description: None,
        inputs: None,
        depends_on: None,
        steps: vec![make_step("step1")],
        success_actions: None,
        failure_actions: None,
        outputs: None,
        parameters: None,
        extensions: Default::default(),
    }
}

fn make_resolved_op() -> arazzo_exec::openapi::ResolvedOperation {
    arazzo_exec::openapi::ResolvedOperation {
        source_name: "petstore".to_string(),
        base_url: "https://api.test.local".to_string(),
        method: "GET".to_string(),
        path: "/users".to_string(),
        operation_id: Some("getUsers".to_string()),
        shape: arazzo_exec::openapi::CompiledOperationShape {
            parameters: vec![],
            request_body_required: None,
            request_body_content_types: None,
        },
    }
}

fn make_policy() -> PolicyConfig {
    use std::collections::BTreeSet;
    PolicyConfig {
        network: arazzo_exec::policy::NetworkConfig {
            allowed_schemes: ["https"].into_iter().map(|s| s.to_string()).collect(),
            allowed_hosts: ["api.test.local"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            allowed_base_urls: BTreeSet::new(),
            redirects: Default::default(),
            deny_private_ip_literals: true,
        },
        limits: Default::default(),
        sensitive_headers: Default::default(),
        allow_secrets_in_url: false,
        per_source: BTreeMap::new(),
    }
}

#[tokio::test]
async fn successful_step_returns_outputs() {
    let store = MockStore;
    let http = MockHttpClient {
        response: HttpResponseParts {
            status: 200,
            headers: BTreeMap::new(),
            body: b"{}".to_vec(),
        },
        fail_with: None,
    };
    let secrets = NoOpSecretsProvider;
    let policy_gate = PolicyGate::new(make_policy());
    let retry = RetryConfig::default();

    let event_sink = MockEventSink;
    let worker = Worker {
        store: &store,
        http: &http,
        secrets: &secrets,
        policy_gate: &policy_gate,
        retry: &retry,
        event_sink: &event_sink,
    };

    let result = arazzo_exec::executor::worker::execute_step_attempt(
        &worker,
        uuid::Uuid::new_v4(),
        "petstore",
        uuid::Uuid::new_v4(),
        &make_step("step1"),
        &make_workflow(),
        &make_resolved_op(),
        &serde_json::json!({}),
        None,
    )
    .await;

    match result {
        StepResult::Succeeded { .. } => {}
        StepResult::Failed { error, .. } => panic!("expected Succeeded, got Failed: {}", error),
        StepResult::Retry { error, .. } => panic!("expected Succeeded, got Retry: {}", error),
    }
}

#[tokio::test]
async fn non_2xx_status_fails_step() {
    let store = MockStore;
    let http = MockHttpClient {
        response: HttpResponseParts {
            status: 404,
            headers: BTreeMap::new(),
            body: b"{}".to_vec(),
        },
        fail_with: None,
    };
    let secrets = NoOpSecretsProvider;
    let policy_gate = PolicyGate::new(make_policy());
    let retry = RetryConfig::default();

    let event_sink = MockEventSink;
    let worker = Worker {
        store: &store,
        http: &http,
        secrets: &secrets,
        policy_gate: &policy_gate,
        retry: &retry,
        event_sink: &event_sink,
    };

    let result = arazzo_exec::executor::worker::execute_step_attempt(
        &worker,
        uuid::Uuid::new_v4(),
        "petstore",
        uuid::Uuid::new_v4(),
        &make_step("step1"),
        &make_workflow(),
        &make_resolved_op(),
        &serde_json::json!({}),
        None,
    )
    .await;

    assert!(matches!(result, StepResult::Failed { end_run: true, .. }));
}

#[tokio::test]
async fn network_error_fails_step() {
    let store = MockStore;
    let http = MockHttpClient {
        response: HttpResponseParts {
            status: 200,
            headers: BTreeMap::new(),
            body: vec![],
        },
        fail_with: Some(HttpError::Timeout),
    };
    let secrets = NoOpSecretsProvider;
    let policy_gate = PolicyGate::new(make_policy());
    let retry = RetryConfig::default();

    let event_sink = MockEventSink;
    let worker = Worker {
        store: &store,
        http: &http,
        secrets: &secrets,
        policy_gate: &policy_gate,
        retry: &retry,
        event_sink: &event_sink,
    };

    let result = arazzo_exec::executor::worker::execute_step_attempt(
        &worker,
        uuid::Uuid::new_v4(),
        "petstore",
        uuid::Uuid::new_v4(),
        &make_step("step1"),
        &make_workflow(),
        &make_resolved_op(),
        &serde_json::json!({}),
        None,
    )
    .await;

    assert!(matches!(result, StepResult::Failed { .. }));
}

#[tokio::test]
async fn missing_base_url_fails_step() {
    let store = MockStore;
    let http = MockHttpClient {
        response: HttpResponseParts {
            status: 200,
            headers: BTreeMap::new(),
            body: vec![],
        },
        fail_with: None,
    };
    let secrets = NoOpSecretsProvider;
    let policy_gate = PolicyGate::new(make_policy());
    let retry = RetryConfig::default();

    let event_sink = MockEventSink;
    let worker = Worker {
        store: &store,
        http: &http,
        secrets: &secrets,
        policy_gate: &policy_gate,
        retry: &retry,
        event_sink: &event_sink,
    };

    let mut op = make_resolved_op();
    op.base_url = String::new();

    let result = arazzo_exec::executor::worker::execute_step_attempt(
        &worker,
        uuid::Uuid::new_v4(),
        "petstore",
        uuid::Uuid::new_v4(),
        &make_step("step1"),
        &make_workflow(),
        &op,
        &serde_json::json!({}),
        None,
    )
    .await;

    match result {
        StepResult::Failed { error, end_run } => {
            assert!(end_run);
            assert!(error["message"].as_str().unwrap().contains("base_url"));
        }
        _ => panic!("expected Failed result"),
    }
}

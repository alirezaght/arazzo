use arazzo_core::types::{FailureAction, FailureActionOrReusable, FailureActionType, Step};
use arazzo_exec::executor::failure::{decide_failure, decide_network_failure};
use arazzo_exec::executor::http::HttpError;
use arazzo_exec::executor::worker::StepResult;
use arazzo_exec::policy::HttpResponseParts;
use arazzo_exec::retry::RetryConfig;
use std::collections::BTreeMap;

fn make_response(status: u16) -> HttpResponseParts {
    HttpResponseParts {
        status,
        headers: BTreeMap::new(),
        body: vec![],
    }
}

fn make_step(step_id: &str) -> Step {
    Step {
        step_id: step_id.to_string(),
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
    }
}

#[test]
fn decide_failure_returns_retry_when_retry_action_present() {
    let mut step = make_step("test");
    step.on_failure = Some(vec![FailureActionOrReusable::Action(FailureAction {
        name: "retry".to_string(),
        action_type: FailureActionType::Retry,
        retry_limit: Some(3u32),
        retry_after_seconds: Some(1.0),
        step_id: None,
        workflow_id: None,
        criteria: None,
        extensions: BTreeMap::new(),
    })]);

    let mut retry_cfg = RetryConfig::default();
    retry_cfg.max_attempts = 5;
    retry_cfg.max_delay = std::time::Duration::from_secs(10);
    retry_cfg.retry_statuses.insert(500);
    let resp = make_response(500);
    let result = decide_failure(&retry_cfg, &step, 1, &resp);

    match result {
        StepResult::Retry { delay_ms, .. } => {
            assert!(delay_ms > 0);
        }
        _ => panic!("expected retry result, got: {:?}", result),
    }
}

#[test]
fn decide_failure_returns_end_when_end_action_present() {
    let mut step = make_step("test");
    step.on_failure = Some(vec![FailureActionOrReusable::Action(FailureAction {
        name: "end".to_string(),
        action_type: FailureActionType::End,
        retry_limit: None,
        retry_after_seconds: None,
        step_id: None,
        workflow_id: None,
        criteria: None,
        extensions: BTreeMap::new(),
    })]);

    let retry_cfg = RetryConfig::default();
    let resp = make_response(500);
    let result = decide_failure(&retry_cfg, &step, 1, &resp);

    match result {
        StepResult::Failed { end_run, .. } => {
            assert!(end_run);
        }
        _ => panic!("expected failed result with end_run=true"),
    }
}

#[test]
fn decide_failure_defaults_to_failed_when_no_actions() {
    let step = make_step("test");

    let retry_cfg = RetryConfig::default();
    let resp = make_response(500);
    let result = decide_failure(&retry_cfg, &step, 1, &resp);

    match result {
        StepResult::Failed { end_run, .. } => {
            assert!(end_run);
        }
        _ => panic!("expected failed result"),
    }
}

#[test]
fn decide_network_failure_returns_retry_when_retry_action_present() {
    let mut step = make_step("test");
    step.on_failure = Some(vec![FailureActionOrReusable::Action(FailureAction {
        name: "retry".to_string(),
        action_type: FailureActionType::Retry,
        retry_limit: Some(3u32),
        retry_after_seconds: Some(1.0),
        step_id: None,
        workflow_id: None,
        criteria: None,
        extensions: BTreeMap::new(),
    })]);

    let mut retry_cfg = RetryConfig::default();
    retry_cfg.max_attempts = 5;
    retry_cfg.max_delay = std::time::Duration::from_secs(10);
    let err = HttpError::Timeout;
    let result = decide_network_failure(&retry_cfg, &step, 1, &err);

    match result {
        StepResult::Retry { delay_ms, .. } => {
            assert!(delay_ms > 0);
        }
        _ => panic!("expected retry result, got: {:?}", result),
    }
}

#[test]
fn decide_network_failure_defaults_to_failed_when_no_retry() {
    let step = make_step("test");

    let retry_cfg = RetryConfig::default();
    let err = HttpError::Network("connection failed".to_string());
    let result = decide_network_failure(&retry_cfg, &step, 1, &err);

    match result {
        StepResult::Failed { end_run, .. } => {
            assert!(end_run);
        }
        _ => panic!("expected failed result"),
    }
}


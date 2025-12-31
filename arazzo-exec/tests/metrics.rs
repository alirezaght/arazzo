use arazzo_exec::executor::{MetricsCollector, RunMetrics};
use arazzo_store::RunStatus;
use uuid::Uuid;

#[test]
fn run_metrics_new() {
    let metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    assert_eq!(metrics.workflow_id, "workflow1");
    assert!(metrics.started_at.is_some());
    assert_eq!(metrics.steps_total, 0);
}

#[test]
fn run_metrics_record_step_success() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    metrics.record_step_success();
    assert_eq!(metrics.steps_total, 1);
    assert_eq!(metrics.steps_succeeded, 1);
    assert_eq!(metrics.steps_failed, 0);
}

#[test]
fn run_metrics_record_step_failure() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    metrics.record_step_failure();
    assert_eq!(metrics.steps_total, 1);
    assert_eq!(metrics.steps_succeeded, 0);
    assert_eq!(metrics.steps_failed, 1);
}

#[test]
fn run_metrics_record_retry() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    metrics.record_retry();
    assert_eq!(metrics.steps_retried, 1);
}

#[test]
fn run_metrics_record_http_events() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    metrics.record_http_request();
    metrics.record_http_request();
    metrics.record_http_error();
    assert_eq!(metrics.http_requests, 2);
    assert_eq!(metrics.http_errors, 1);
}

#[test]
fn run_metrics_finish() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.finish(RunStatus::Succeeded);
    assert_eq!(metrics.status, "succeeded");
    assert!(metrics.finished_at.is_some());
    assert!(metrics.total_duration.is_some());
}

#[tokio::test]
async fn metrics_collector_new() {
    let collector = MetricsCollector::new(Uuid::new_v4(), "workflow1".to_string());
    let metrics = collector.get_metrics().await;
    assert_eq!(metrics.workflow_id, "workflow1");
}

#[tokio::test]
async fn metrics_collector_record_events() {
    let collector = MetricsCollector::new(Uuid::new_v4(), "workflow1".to_string());
    collector.record_step_success().await;
    collector.record_step_failure().await;
    collector.record_retry().await;
    collector.record_http_request().await;
    collector.record_http_error().await;
    collector.record_policy_denial().await;

    let metrics = collector.get_metrics().await;
    assert_eq!(metrics.steps_succeeded, 1);
    assert_eq!(metrics.steps_failed, 1);
    assert_eq!(metrics.steps_retried, 1);
    assert_eq!(metrics.http_requests, 1);
    assert_eq!(metrics.http_errors, 1);
    assert_eq!(metrics.policy_denials, 1);
}

#[test]
fn metrics_to_json() {
    let mut metrics = RunMetrics::new(Uuid::new_v4(), "workflow1".to_string());
    metrics.record_step_success();
    metrics.record_http_request();
    metrics.finish(RunStatus::Succeeded);

    let json = metrics.to_json();
    assert_eq!(json["workflow_id"], "workflow1");
    assert_eq!(json["status"], "succeeded");
    assert_eq!(json["steps"]["total"], 1);
    assert_eq!(json["steps"]["succeeded"], 1);
    assert_eq!(json["http"]["requests"], 1);
}


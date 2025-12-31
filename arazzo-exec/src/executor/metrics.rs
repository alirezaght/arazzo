use crate::executor::{Event, EventSink};
use arazzo_store::RunStatus;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct RunMetrics {
    pub run_id: uuid::Uuid,
    pub workflow_id: String,
    pub status: String,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub total_duration: Option<Duration>,
    pub steps_total: usize,
    pub steps_succeeded: usize,
    pub steps_failed: usize,
    pub steps_retried: usize,
    pub http_requests: usize,
    pub http_errors: usize,
    pub policy_denials: usize,
}

impl RunMetrics {
    pub fn new(run_id: uuid::Uuid, workflow_id: String) -> Self {
        Self {
            run_id,
            workflow_id,
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn record_step_success(&mut self) {
        self.steps_succeeded += 1;
        self.steps_total += 1;
    }

    pub fn record_step_failure(&mut self) {
        self.steps_failed += 1;
        self.steps_total += 1;
    }

    pub fn record_retry(&mut self) {
        self.steps_retried += 1;
    }

    pub fn record_http_request(&mut self) {
        self.http_requests += 1;
    }

    pub fn record_http_error(&mut self) {
        self.http_errors += 1;
    }

    pub fn record_policy_denial(&mut self) {
        self.policy_denials += 1;
    }

    pub fn finish(&mut self, status: RunStatus) {
        self.status = status.as_str().to_string();
        self.finished_at = Some(Instant::now());
        if let (Some(started), Some(finished)) = (self.started_at, self.finished_at) {
            self.total_duration = Some(finished.duration_since(started));
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "run_id": self.run_id.to_string(),
            "workflow_id": self.workflow_id,
            "status": self.status,
            "duration_ms": self.total_duration.map(|d| d.as_millis() as u64),
            "steps": {
                "total": self.steps_total,
                "succeeded": self.steps_succeeded,
                "failed": self.steps_failed,
                "retried": self.steps_retried,
            },
            "http": {
                "requests": self.http_requests,
                "errors": self.http_errors,
            },
            "policy_denials": self.policy_denials,
        })
    }
}

pub struct MetricsCollector {
    metrics: Arc<Mutex<RunMetrics>>,
}

impl MetricsCollector {
    pub fn new(run_id: uuid::Uuid, workflow_id: String) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(RunMetrics::new(run_id, workflow_id))),
        }
    }

    pub async fn record_step_success(&self) {
        self.metrics.lock().await.record_step_success();
    }

    pub async fn record_step_failure(&self) {
        self.metrics.lock().await.record_step_failure();
    }

    pub async fn record_retry(&self) {
        self.metrics.lock().await.record_retry();
    }

    pub async fn record_http_request(&self) {
        self.metrics.lock().await.record_http_request();
    }

    pub async fn record_http_error(&self) {
        self.metrics.lock().await.record_http_error();
    }

    pub async fn record_policy_denial(&self) {
        self.metrics.lock().await.record_policy_denial();
    }

    pub async fn finish(&self, status: RunStatus) {
        self.metrics.lock().await.finish(status);
    }

    pub async fn get_metrics(&self) -> RunMetrics {
        self.metrics.lock().await.clone()
    }
}

pub struct MetricsEventSink {
    collector: Arc<MetricsCollector>,
    base: Arc<dyn EventSink>,
}

impl MetricsEventSink {
    pub fn new(collector: Arc<MetricsCollector>, base: Arc<dyn EventSink>) -> Self {
        Self { collector, base }
    }
}

#[async_trait]
impl EventSink for MetricsEventSink {
    async fn emit(&self, event: Event) {
        // Update metrics based on event
        match &event {
            Event::StepSucceeded { .. } => {
                self.collector.record_step_success().await;
            }
            Event::StepFailed { .. } => {
                self.collector.record_step_failure().await;
            }
            Event::StepRetryScheduled { .. } => {
                self.collector.record_retry().await;
            }
            Event::AttemptStarted { .. } => {
                self.collector.record_http_request().await;
            }
            Event::AttemptFinished { succeeded, .. } => {
                if !succeeded {
                    self.collector.record_http_error().await;
                }
            }
            Event::PolicyDenied { .. } => {
                self.collector.record_policy_denial().await;
            }
            Event::RunFinished { status, .. } => {
                self.collector.finish(*status).await;
            }
            _ => {}
        }

        // Forward to base sink
        self.base.emit(event).await;
    }
}

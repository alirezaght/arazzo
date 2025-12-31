use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use arazzo_exec::executor::events::{Event, EventSink, NoOpEventSink};
use arazzo_exec::executor::http::{HttpClient, HttpError};
use arazzo_exec::executor::webhook::WebhookEventSink;
use arazzo_exec::policy::{HttpRequestParts, HttpResponseParts};
use arazzo_store::RunStatus;
use async_trait::async_trait;
use std::collections::BTreeMap;

struct MockHttpClient {
    requests: Arc<tokio::sync::Mutex<Vec<HttpRequestParts>>>,
}

#[async_trait]
impl HttpClient for MockHttpClient {
    async fn send(
        &self,
        req: HttpRequestParts,
        _timeout: Duration,
        _max_response_bytes: usize,
    ) -> Result<HttpResponseParts, HttpError> {
        self.requests.lock().await.push(req);
        Ok(HttpResponseParts {
            status: 200,
            headers: BTreeMap::new(),
            body: vec![],
        })
    }
}

#[tokio::test]
async fn webhook_sink_sends_on_run_finished() {
    let requests = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let http = Arc::new(MockHttpClient {
        requests: requests.clone(),
    });
    let base = Arc::new(NoOpEventSink);
    let sink = WebhookEventSink::new("https://example.com/webhook".to_string(), http, base);

    sink.emit(Event::RunFinished {
        run_id: Uuid::new_v4(),
        status: RunStatus::Succeeded,
    })
    .await;

    tokio::time::sleep(Duration::from_millis(100)).await;
    let reqs = requests.lock().await;
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].method, "POST");
    assert_eq!(reqs[0].url.to_string(), "https://example.com/webhook");
}

#[tokio::test]
async fn webhook_sink_ignores_non_finished_events() {
    let requests = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let http = Arc::new(MockHttpClient {
        requests: requests.clone(),
    });
    let base = Arc::new(NoOpEventSink);
    let sink = WebhookEventSink::new("https://example.com/webhook".to_string(), http, base);

    sink.emit(Event::RunStarted {
        run_id: Uuid::new_v4(),
        workflow_id: "test".to_string(),
    })
    .await;

    sink.emit(Event::StepStarted {
        run_id: Uuid::new_v4(),
        step_id: "step1".to_string(),
    })
    .await;

    tokio::time::sleep(Duration::from_millis(100)).await;
    let reqs = requests.lock().await;
    assert_eq!(reqs.len(), 0);
}

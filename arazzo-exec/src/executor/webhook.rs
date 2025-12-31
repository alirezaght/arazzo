use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::executor::http::HttpClient;
use crate::executor::{Event, EventSink};
use crate::policy::HttpRequestParts;

pub struct WebhookEventSink {
    url: String,
    http: Arc<dyn HttpClient>,
    base: Arc<dyn EventSink>,
}

impl WebhookEventSink {
    pub fn new(url: String, http: Arc<dyn HttpClient>, base: Arc<dyn EventSink>) -> Self {
        Self { url, http, base }
    }
}

#[async_trait]
impl EventSink for WebhookEventSink {
    async fn emit(&self, event: Event) {
        self.base.emit(event.clone()).await;

        let payload = match &event {
            Event::RunFinished { run_id, status } => Some(json!({
                "type": "run.finished",
                "run_id": run_id.to_string(),
                "status": status.as_str(),
            })),
            _ => None,
        };

        if let Some(payload) = payload {
            let body = serde_json::to_vec(&payload).unwrap_or_default();
            let url = match url::Url::parse(&self.url) {
                Ok(u) => u,
                Err(_) => return,
            };

            let req = HttpRequestParts {
                method: "POST".to_string(),
                url,
                headers: std::collections::BTreeMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )]),
                body,
            };

            let http = self.http.clone();
            tokio::spawn(async move {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    http.send(req, std::time::Duration::from_secs(5), 1024 * 1024),
                )
                .await;
            });
        }
    }
}

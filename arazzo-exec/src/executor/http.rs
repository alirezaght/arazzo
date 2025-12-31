use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;

use crate::policy::{HttpRequestParts, HttpResponseParts};

#[derive(Debug, Clone, thiserror::Error)]
pub enum HttpError {
    #[error("timeout")]
    Timeout,
    #[error("connect/dns/tls error: {0}")]
    Network(String),
    #[error("response too large (>{max_bytes} bytes)")]
    ResponseTooLarge { max_bytes: usize },
    #[error("http error: {0}")]
    Other(String),
}

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn send(
        &self,
        req: HttpRequestParts,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<HttpResponseParts, HttpError>;
}

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        // Redirect policy is handled by policy; keep reqwest redirects disabled by default.
        // Client creation should never fail in practice, but if it does, we'll get a better error
        // when trying to use it rather than panicking at initialization.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("arazzo-exec/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|e| {
                panic!("failed to create reqwest HTTP client: {e}. This is a bug - please report it.");
            });
        Self { client }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn send(
        &self,
        req: HttpRequestParts,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<HttpResponseParts, HttpError> {
        let method: reqwest::Method = req.method.parse().map_err(|e: <reqwest::Method as std::str::FromStr>::Err| HttpError::Other(e.to_string()))?;
        let mut rb = self
            .client
            .request(method, req.url)
            .timeout(timeout);

        for (k, v) in req.headers {
            rb = rb.header(k, v);
        }

        rb = rb.body(req.body);

        let resp = rb.send().await.map_err(map_reqwest_error)?;
        let status = resp.status().as_u16();

        let mut headers = BTreeMap::new();
        for (k, v) in resp.headers().iter() {
            if let Ok(s) = v.to_str() {
                headers.insert(k.to_string(), s.to_string());
            }
        }

        // Read response body with size cap.
        let body = resp.bytes().await.map_err(map_reqwest_error)?;
        if body.len() > max_response_bytes {
            return Err(HttpError::ResponseTooLarge { max_bytes: max_response_bytes });
        }
        let body = body.to_vec();

        Ok(HttpResponseParts { status, headers, body })
    }
}

fn map_reqwest_error(e: reqwest::Error) -> HttpError {
    if e.is_timeout() {
        return HttpError::Timeout;
    }
    if e.is_connect() || e.is_request() {
        return HttpError::Network(e.to_string());
    }
    HttpError::Other(e.to_string())
}


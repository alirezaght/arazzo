use std::time::Duration;

use arazzo_exec::executor::http::{HttpClient, HttpError, ReqwestHttpClient};
use arazzo_exec::policy::HttpRequestParts;

#[tokio::test]
async fn http_client_sends_get_request() {
    let client = ReqwestHttpClient::default();
    let req = HttpRequestParts {
        method: "GET".to_string(),
        url: url::Url::parse("https://httpbin.org/get").unwrap(),
        headers: std::collections::BTreeMap::new(),
        body: vec![],
    };

    let result = client.send(req, Duration::from_secs(10), 1024 * 1024).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn http_client_sends_post_request() {
    let client = ReqwestHttpClient::default();
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    let req = HttpRequestParts {
        method: "POST".to_string(),
        url: url::Url::parse("https://httpbin.org/post").unwrap(),
        headers,
        body: b"{\"test\":\"value\"}".to_vec(),
    };

    let result = client.send(req, Duration::from_secs(10), 1024 * 1024).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn http_client_handles_timeout() {
    let client = ReqwestHttpClient::default();
    let req = HttpRequestParts {
        method: "GET".to_string(),
        url: url::Url::parse("https://httpbin.org/delay/5").unwrap(),
        headers: std::collections::BTreeMap::new(),
        body: vec![],
    };

    let result = client.send(req, Duration::from_secs(1), 1024 * 1024).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        HttpError::Timeout => {}
        _ => panic!("expected timeout error"),
    }
}

#[tokio::test]
async fn http_client_enforces_response_size_limit() {
    let client = ReqwestHttpClient::default();
    let req = HttpRequestParts {
        method: "GET".to_string(),
        url: url::Url::parse("https://httpbin.org/bytes/1000").unwrap(),
        headers: std::collections::BTreeMap::new(),
        body: vec![],
    };

    let result = client.send(req, Duration::from_secs(10), 100).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        HttpError::ResponseTooLarge { max_bytes } => {
            assert_eq!(max_bytes, 100);
        }
        _ => panic!("expected response too large error"),
    }
}

#[tokio::test]
async fn http_client_handles_invalid_url() {
    let client = ReqwestHttpClient::default();
    let req = HttpRequestParts {
        method: "GET".to_string(),
        url: url::Url::parse("https://invalid-domain-that-does-not-exist-12345.com").unwrap(),
        headers: std::collections::BTreeMap::new(),
        body: vec![],
    };

    let result = client.send(req, Duration::from_secs(5), 1024 * 1024).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        HttpError::Network(_) => {}
        _ => panic!("expected network error"),
    }
}

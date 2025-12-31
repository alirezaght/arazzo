use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use arazzo_exec::policy::{HttpRequestParts, PolicyConfig, PolicyGate};
use arazzo_exec::retry::{RetryConfig, decide_retry, RetryDecision, RetryReason};

fn req(url: &str, body_len: usize) -> HttpRequestParts {
    HttpRequestParts {
        method: "GET".to_string(),
        url: url::Url::parse(url).unwrap(),
        headers: BTreeMap::new(),
        body: vec![0u8; body_len],
    }
}

#[test]
fn policy_denies_when_host_allowlist_empty() {
    let gate = PolicyGate::new(PolicyConfig::default());
    let r = req("https://example.com/", 0);
    let err = gate.apply_request("store", &r, &[], false).unwrap_err();
    assert!(format!("{err}").contains("disallowed host"));
}

#[test]
fn policy_allows_https_and_allowlisted_host() {
    let mut cfg = PolicyConfig::default();
    cfg.network.allowed_hosts.insert("example.com".to_string());
    let gate = PolicyGate::new(cfg);
    let r = req("https://api.example.com/orders", 0);
    let ok = gate.apply_request("store", &r, &[], false).unwrap();
    assert_eq!(ok.method, "GET");
}

#[test]
fn policy_denies_http_by_default() {
    let mut cfg = PolicyConfig::default();
    cfg.network.allowed_hosts.insert("example.com".to_string());
    let gate = PolicyGate::new(cfg);
    let r = req("http://example.com/", 0);
    let err = gate.apply_request("store", &r, &[], false).unwrap_err();
    assert!(format!("{err}").contains("disallowed URL scheme"));
}

#[test]
fn policy_enforces_request_body_size() {
    let mut cfg = PolicyConfig::default();
    cfg.network.allowed_hosts.insert("example.com".to_string());
    cfg.limits.request.max_body_bytes = 10;
    let gate = PolicyGate::new(cfg);
    let r = req("https://example.com/", 11);
    let err = gate.apply_request("store", &r, &[], false).unwrap_err();
    assert!(format!("{err}").contains("request body exceeds"));
}

#[test]
fn retry_uses_retry_after_header_over_backoff() {
    let cfg = RetryConfig::default();
    let mut headers = BTreeMap::new();
    headers.insert("Retry-After".to_string(), "5".to_string());

    let d = decide_retry(
        &cfg,
        1,
        Some(5),
        Some(1),
        false,
        Some(429),
        Some(&headers),
        false,
        SystemTime::UNIX_EPOCH,
        || 123,
    );
    assert_eq!(
        d,
        RetryDecision::RetryAfter {
            delay: Duration::from_secs(5),
            reason: RetryReason::RetryAfterHeader
        }
    );
}

#[test]
fn retry_stops_on_policy_failure() {
    let cfg = RetryConfig::default();
    let d = decide_retry(
        &cfg,
        1,
        Some(5),
        None,
        true,
        Some(503),
        None,
        false,
        SystemTime::UNIX_EPOCH,
        || 0,
    );
    assert!(matches!(d, RetryDecision::Stop { reason: RetryReason::PolicyFailure }));
}


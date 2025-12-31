use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use arazzo_exec::retry::{RetryHeadersConfig, RetryVendorHeader, VendorHeaderKind};
use arazzo_exec::retry::parse_retry_after;

#[test]
fn parse_retry_after_delta_seconds() {
    let mut headers = BTreeMap::new();
    headers.insert("Retry-After".to_string(), "5".to_string());
    let cfg = RetryHeadersConfig::default();
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Duration::from_secs(5));
}

#[test]
fn parse_retry_after_http_date() {
    let mut headers = BTreeMap::new();
    let future = SystemTime::now() + Duration::from_secs(10);
    let http_date = httpdate::fmt_http_date(future);
    headers.insert("Retry-After".to_string(), http_date);
    let cfg = RetryHeadersConfig::default();
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    let delay = result.unwrap();
    assert!(delay.as_secs() >= 9 && delay.as_secs() <= 11);
}

#[test]
fn parse_retry_after_case_insensitive() {
    let mut headers = BTreeMap::new();
    headers.insert("retry-after".to_string(), "3".to_string());
    let cfg = RetryHeadersConfig::default();
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Duration::from_secs(3));
}

#[test]
fn parse_retry_after_vendor_header_delta_seconds() {
    let mut headers = BTreeMap::new();
    headers.insert("X-RateLimit-Reset".to_string(), "7".to_string());
    let mut cfg = RetryHeadersConfig::default();
    cfg.vendor_headers.push(arazzo_exec::retry::RetryVendorHeader {
        name: "X-RateLimit-Reset".to_string(),
        kind: VendorHeaderKind::DeltaSeconds,
    });
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Duration::from_secs(7));
}

#[test]
fn parse_retry_after_vendor_header_unix_seconds() {
    let mut headers = BTreeMap::new();
    let future = SystemTime::now() + Duration::from_secs(15);
    let unix_secs = future.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    headers.insert("X-Reset-At".to_string(), unix_secs.to_string());
    let mut cfg = RetryHeadersConfig::default();
    cfg.vendor_headers.push(arazzo_exec::retry::RetryVendorHeader {
        name: "X-Reset-At".to_string(),
        kind: VendorHeaderKind::UnixSeconds,
    });
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    let delay = result.unwrap();
    assert!(delay.as_secs() >= 14 && delay.as_secs() <= 16);
}

#[test]
fn parse_retry_after_returns_none_when_missing() {
    let headers = BTreeMap::new();
    let cfg = RetryHeadersConfig::default();
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_none());
}

#[test]
fn parse_retry_after_standard_header_takes_precedence() {
    let mut headers = BTreeMap::new();
    headers.insert("Retry-After".to_string(), "2".to_string());
    headers.insert("X-Custom-Retry".to_string(), "10".to_string());
    let mut cfg = RetryHeadersConfig::default();
    cfg.vendor_headers.push(arazzo_exec::retry::RetryVendorHeader {
        name: "X-Custom-Retry".to_string(),
        kind: VendorHeaderKind::DeltaSeconds,
    });
    let now = SystemTime::now();

    let result = parse_retry_after(&headers, &cfg, now);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Duration::from_secs(2));
}


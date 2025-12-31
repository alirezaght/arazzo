use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use httpdate::parse_http_date;

use crate::retry::config::{RetryHeadersConfig, VendorHeaderKind};

pub fn parse_retry_after(
    headers: &BTreeMap<String, String>,
    cfg: &RetryHeadersConfig,
    now: SystemTime,
) -> Option<Duration> {
    // Standard header wins.
    if let Some(v) = get_header_ci(headers, "retry-after") {
        if let Some(d) = parse_retry_after_value(v, now) {
            return Some(d);
        }
    }

    for vh in &cfg.vendor_headers {
        if let Some(v) = get_header_ci(headers, &vh.name) {
            if let Some(d) = parse_vendor_value(v, vh.kind, now) {
                return Some(d);
            }
        }
    }
    None
}

fn parse_retry_after_value(v: &str, now: SystemTime) -> Option<Duration> {
    let v = v.trim();
    if let Ok(secs) = v.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    // HTTP-date
    let dt = parse_http_date(v).ok()?;
    dt.duration_since(now).ok()
}

fn parse_vendor_value(v: &str, kind: VendorHeaderKind, now: SystemTime) -> Option<Duration> {
    let v = v.trim();
    match kind {
        VendorHeaderKind::DeltaSeconds => v.parse::<u64>().ok().map(Duration::from_secs),
        VendorHeaderKind::UnixSeconds => {
            let ts = v.parse::<u64>().ok()?;
            let dt = SystemTime::UNIX_EPOCH + Duration::from_secs(ts);
            dt.duration_since(now).ok()
        }
        VendorHeaderKind::HttpDate => {
            let dt = parse_http_date(v).ok()?;
            dt.duration_since(now).ok()
        }
    }
}

fn get_header_ci<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

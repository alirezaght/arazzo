use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use crate::retry::config::RetryConfig;
use crate::retry::headers::parse_retry_after;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    RetryAfter { delay: Duration, reason: RetryReason },
    Stop { reason: RetryReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryReason {
    NotRetryable,
    AttemptsExhausted,
    PolicyFailure,
    NetworkFailure,
    HttpStatus(u16),
    RetryAfterHeader,
    Backoff,
}

/// Decide if we should retry and how long to wait.
///
/// - `attempt_no`: 1-based attempt number for this step.
/// - `arazzo_retry_limit`: from the matched Arazzo failure action; if None, assume 1 retry.
/// - `arazzo_retry_after_seconds`: from Arazzo failure action; used only if header absent.
/// - `policy_failed`: if true, never retry.
/// - `http_status`: status code if available.
/// - `response_headers`: headers if available.
/// - `network_failed`: if true, treat as retryable network failure (subject to limits).
/// - `now`: time source for parsing HTTP-date retry-after.
/// - `rand_u64`: RNG for full jitter.
pub fn decide_retry(
    cfg: &RetryConfig,
    attempt_no: usize,
    arazzo_retry_limit: Option<usize>,
    arazzo_retry_after_seconds: Option<u64>,
    policy_failed: bool,
    http_status: Option<u16>,
    response_headers: Option<&BTreeMap<String, String>>,
    network_failed: bool,
    now: SystemTime,
    rand_u64: impl Fn() -> u64,
) -> RetryDecision {
    if policy_failed {
        return RetryDecision::Stop {
            reason: RetryReason::PolicyFailure,
        };
    }

    let arazzo_limit = arazzo_retry_limit.unwrap_or(1);
    let max_attempts = cfg.max_attempts.min(arazzo_limit.max(1) + 1); // attempts = initial + retries
    if attempt_no >= max_attempts {
        return RetryDecision::Stop {
            reason: RetryReason::AttemptsExhausted,
        };
    }

    if let Some(status) = http_status {
        if !cfg.retry_statuses.contains(&status) {
            return RetryDecision::Stop {
                reason: RetryReason::HttpStatus(status),
            };
        }
    } else if !network_failed {
        return RetryDecision::Stop {
            reason: RetryReason::NotRetryable,
        };
    }

    // Retry-After header wins.
    if let Some(h) = response_headers {
        if let Some(delay) = parse_retry_after(h, &cfg.headers, now) {
            return RetryDecision::RetryAfter {
                delay: clamp(delay, cfg.max_delay),
                reason: RetryReason::RetryAfterHeader,
            };
        }
    }

    // Arazzo retryAfter as seconds (if provided).
    if let Some(secs) = arazzo_retry_after_seconds {
        let d = Duration::from_secs(secs);
        return RetryDecision::RetryAfter {
            delay: clamp(d, cfg.max_delay),
            reason: RetryReason::Backoff,
        };
    }

    // Exponential backoff: base * factor^(attempt_no-1), with full jitter.
    let exp = (attempt_no.saturating_sub(1)) as i32;
    let raw = (cfg.base_delay.as_millis() as f64) * cfg.factor.powi(exp);
    let raw_ms = raw.min(cfg.max_delay.as_millis() as f64).max(0.0) as u64;

    let jitter_ms = if raw_ms == 0 { 0 } else { rand_u64() % (raw_ms + 1) };
    RetryDecision::RetryAfter {
        delay: Duration::from_millis(jitter_ms),
        reason: http_status.map(RetryReason::HttpStatus).unwrap_or(RetryReason::NetworkFailure),
    }
}

fn clamp(delay: Duration, max: Duration) -> Duration {
    if delay > max { max } else { delay }
}


use std::time::SystemTime;

use arazzo_core::types::{FailureActionOrReusable, FailureActionType, Step};
use serde_json::json;

use crate::executor::http::HttpError;
use crate::executor::worker::StepResult;
use crate::policy::HttpResponseParts;
use crate::retry::{RetryConfig, RetryDecision, decide_retry};

pub fn decide_failure(
    retry_cfg: &RetryConfig,
    step: &Step,
    attempt_no: usize,
    resp: &HttpResponseParts,
) -> StepResult {
    let actions = step.on_failure.as_deref().unwrap_or(&[]);
    for a in actions {
        if let FailureActionOrReusable::Action(a) = a {
            match a.action_type {
                FailureActionType::Retry => {
                    let dec = decide_retry(
                        retry_cfg,
                        attempt_no,
                        a.retry_limit.map(|v| v as usize),
                        a.retry_after_seconds.map(|f| f as u64),
                        false,
                        Some(resp.status),
                        Some(&resp.headers),
                        false,
                        SystemTime::now(),
                        || fastrand::u64(..),
                    );
                    if let RetryDecision::RetryAfter { delay, .. } = dec {
                        return StepResult::Retry {
                            delay_ms: delay.as_millis() as i64,
                            error: json!({"type":"http","status":resp.status}),
                        };
                    }
                }
                FailureActionType::End => {
                    return StepResult::Failed {
                        error: json!({"type":"http","status":resp.status}),
                        end_run: true,
                    };
                }
                _ => {}
            }
        }
    }
    StepResult::Failed {
        error: json!({"type":"http","status":resp.status}),
        end_run: true,
    }
}

pub fn decide_network_failure(
    retry_cfg: &RetryConfig,
    step: &Step,
    attempt_no: usize,
    err: &HttpError,
) -> StepResult {
    let actions = step.on_failure.as_deref().unwrap_or(&[]);
    for a in actions {
        if let FailureActionOrReusable::Action(a) = a {
            if a.action_type == FailureActionType::Retry {
                let dec = decide_retry(
                    retry_cfg,
                    attempt_no,
                    a.retry_limit.map(|v| v as usize),
                    a.retry_after_seconds.map(|f| f as u64),
                    false,
                    None,
                    None,
                    true,
                    SystemTime::now(),
                    || fastrand::u64(..),
                );
                if let RetryDecision::RetryAfter { delay, .. } = dec {
                    return StepResult::Retry {
                        delay_ms: delay.as_millis() as i64,
                        error: json!({"type":"network","message":err.to_string()}),
                    };
                }
            }
        }
    }
    StepResult::Failed {
        error: json!({"type":"network","message":err.to_string()}),
        end_run: true,
    }
}


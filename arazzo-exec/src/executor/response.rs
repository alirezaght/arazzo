use arazzo_core::types::Step;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::executor::criteria;
use crate::executor::eval::{eval_value, EvalContext, ResponseContext};
use crate::policy::{HttpResponseParts, ResponseGateResult};

pub fn parse_body_json(resp: &HttpResponseParts) -> Option<JsonValue> {
    let s = std::str::from_utf8(&resp.body).ok()?;
    serde_json::from_str(s).ok()
}

pub fn evaluate_success(step: &Step, resp: &ResponseContext<'_>) -> bool {
    let Some(ref crit) = step.success_criteria else {
        return (200..300).contains(&resp.status);
    };
    criteria::evaluate_success(crit, resp)
}

pub async fn compute_outputs(
    store: &dyn arazzo_store::StateStore,
    run_id: Uuid,
    inputs: &JsonValue,
    step: &Step,
    resp: &ResponseContext<'_>,
) -> JsonValue {
    let mut map = serde_json::Map::new();
    if let Some(outputs) = &step.outputs {
        for (k, expr) in outputs {
            let ctx = EvalContext {
                run_id,
                inputs,
                store,
                response: Some(resp.clone()),
            };
            let v = eval_value(&JsonValue::String(expr.clone()), &ctx)
                .await
                .unwrap_or(JsonValue::Null);
            map.insert(k.clone(), v);
        }
    }
    JsonValue::Object(map)
}

pub fn request_to_json(r: &crate::policy::RequestGateResult) -> JsonValue {
    serde_json::json!({
        "method": r.method,
        "url": r.url,
        "headers": r.headers.headers,
        "body": String::from_utf8_lossy(&r.body.bytes).to_string(),
        "body_truncated": r.body.truncated,
    })
}

pub fn response_to_json(r: &ResponseGateResult) -> JsonValue {
    serde_json::json!({
        "status": r.status,
        "headers": r.headers.headers,
        "body": String::from_utf8_lossy(&r.body.bytes).to_string(),
        "body_truncated": r.body.truncated,
    })
}

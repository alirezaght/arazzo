use std::collections::BTreeMap;

use arazzo_core::expressions::{parse_runtime_expr, parse_template, RuntimeExpr, Segment};
use serde_json::Value as JsonValue;

use arazzo_store::StateStore;
use uuid::Uuid;

#[derive(Clone)]
pub struct EvalContext<'a> {
    pub run_id: Uuid,
    pub inputs: &'a JsonValue,
    pub store: &'a dyn StateStore,
    pub response: Option<ResponseContext<'a>>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ResponseContext<'a> {
    pub status: u16,
    pub headers: &'a BTreeMap<String, String>,
    pub body: &'a [u8],
    pub body_json: Option<JsonValue>,
}

pub async fn eval_value(value: &JsonValue, ctx: &EvalContext<'_>) -> Result<JsonValue, String> {
    match value {
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => Ok(value.clone()),
        JsonValue::String(s) => eval_string(s, ctx).await,
        JsonValue::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for v in arr {
                out.push(Box::pin(eval_value(v, ctx)).await?);
            }
            Ok(JsonValue::Array(out))
        }
        JsonValue::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), Box::pin(eval_value(v, ctx)).await?);
            }
            Ok(JsonValue::Object(out))
        }
    }
}

async fn eval_string(s: &str, ctx: &EvalContext<'_>) -> Result<JsonValue, String> {
    let trimmed = s.trim();
    if trimmed.starts_with('$') {
        return eval_runtime_expr(trimmed, ctx).await;
    }

    // Embedded template: replace each `{ $expr }` segment into string.
    let tpl = parse_template(s).map_err(|e| e.to_string())?;
    if tpl.segments.len() == 1 {
        if let Segment::Literal(lit) = &tpl.segments[0] {
            return Ok(JsonValue::String(lit.clone()));
        }
    }

    let mut out = String::new();
    for seg in tpl.segments {
        match seg {
            Segment::Literal(l) => out.push_str(&l),
            Segment::Expr(e) => {
                let v = eval_runtime_expr(&e, ctx).await?;
                match v {
                    JsonValue::String(s) => out.push_str(&s),
                    JsonValue::Number(n) => out.push_str(&n.to_string()),
                    JsonValue::Bool(b) => out.push_str(if b { "true" } else { "false" }),
                    JsonValue::Null => {}
                    other => out.push_str(&other.to_string()),
                }
            }
        }
    }
    Ok(JsonValue::String(out))
}

async fn eval_runtime_expr(expr: &str, ctx: &EvalContext<'_>) -> Result<JsonValue, String> {
    let parsed = parse_runtime_expr(expr).map_err(|e| e.to_string())?;
    match parsed {
        RuntimeExpr::Inputs(np) => {
            let mut cur = ctx.inputs;
            cur = cur
                .get(&np.root)
                .ok_or_else(|| format!("missing input: {}", np.root))?;
            for seg in np.rest {
                cur = cur
                    .get(&seg)
                    .ok_or_else(|| format!("missing input path: {}", seg))?;
            }
            Ok(cur.clone())
        }
        RuntimeExpr::Steps(np) => {
            // Only support `$steps.<stepId>.outputs.<name>` plus optional pointer.
            if np.rest.first().map(|s| s.as_str()) != Some("outputs") {
                return Err("only $steps.<id>.outputs.* is supported".to_string());
            }
            let out_name = np
                .rest
                .get(1)
                .ok_or_else(|| "missing output name".to_string())?;
            let outputs = ctx
                .store
                .get_step_outputs(ctx.run_id, &np.root)
                .await
                .map_err(|e| e.to_string())?;
            let mut cur = outputs
                .get(out_name)
                .ok_or_else(|| format!("missing step output: {}", out_name))?
                .clone();
            if let Some(ptr) = np.pointer {
                if let Some(v) = cur.pointer(ptr.as_str()) {
                    cur = v.clone();
                }
            }
            Ok(cur)
        }
        RuntimeExpr::StatusCode => Ok(JsonValue::Number(
            ctx.response.as_ref().map(|r| r.status).unwrap_or(0).into(),
        )),
        RuntimeExpr::Response(source) => {
            let r = ctx
                .response
                .as_ref()
                .ok_or_else(|| "no response context".to_string())?;
            match source {
                arazzo_core::expressions::Source::Header(h) => {
                    let v = r
                        .headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(&h))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    Ok(JsonValue::String(v))
                }
                arazzo_core::expressions::Source::Body { pointer } => {
                    let json = r
                        .body_json
                        .clone()
                        .ok_or_else(|| "response body is not JSON".to_string())?;
                    if let Some(ptr) = pointer {
                        Ok(json
                            .pointer(ptr.as_str())
                            .cloned()
                            .unwrap_or(JsonValue::Null))
                    } else {
                        Ok(json)
                    }
                }
                _ => Err("unsupported response source".to_string()),
            }
        }
        _ => Err("unsupported runtime expression".to_string()),
    }
}

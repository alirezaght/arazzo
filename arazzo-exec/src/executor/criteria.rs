use arazzo_core::expressions::{RuntimeExpr, Source, parse_runtime_expr};
use arazzo_core::types::{Criterion, CriterionType, KnownCriterionType};
use regex::Regex;
use serde_json::Value as JsonValue;
use serde_json_path::JsonPath;

use super::eval::ResponseContext;

pub fn evaluate_success(criteria: &[Criterion], resp: &ResponseContext<'_>) -> bool {
    if criteria.is_empty() {
        return (200..300).contains(&resp.status);
    }
    for c in criteria {
        if !evaluate_criterion(c, resp) {
            return false;
        }
    }
    true
}

fn evaluate_criterion(c: &Criterion, resp: &ResponseContext<'_>) -> bool {
    let criterion_type = c.r#type.as_ref().map(|t| match t {
        CriterionType::Known(k) => k.clone(),
        CriterionType::Custom(custom) => match custom.r#type {
            arazzo_core::types::CriterionExpressionLanguage::Jsonpath => KnownCriterionType::Jsonpath,
            arazzo_core::types::CriterionExpressionLanguage::Xpath => KnownCriterionType::Xpath,
        },
    });

    match criterion_type {
        None | Some(KnownCriterionType::Simple) => evaluate_simple(c, resp),
        Some(KnownCriterionType::Jsonpath) => evaluate_jsonpath(c, resp),
        Some(KnownCriterionType::Regex) => evaluate_regex(c, resp),
        Some(KnownCriterionType::Xpath) => false, // XPath not implemented
    }
}

fn evaluate_simple(c: &Criterion, resp: &ResponseContext<'_>) -> bool {
    let cond = c.condition.trim();

    // Parse as: <expr> <op> <literal>
    let ops = ["==", "!=", "<=", ">=", "<", ">"];
    for op in ops {
        if let Some((lhs, rhs)) = cond.split_once(op) {
            let lhs_val = resolve_runtime_expr(lhs.trim(), resp);
            let rhs_val = parse_literal(rhs.trim());
            return compare_values(&lhs_val, &rhs_val, op);
        }
    }

    false
}

fn evaluate_jsonpath(c: &Criterion, resp: &ResponseContext<'_>) -> bool {
    let context_expr = match &c.context {
        Some(ctx) => ctx.as_str(),
        None => return false,
    };

    let context_json = resolve_runtime_expr(context_expr, resp);
    if context_json.is_null() {
        return false;
    }

    let condition = c.condition.trim();

    // For filter expressions $[?...], we need the context to be an array.
    // If it's an object, wrap it in an array so filters work as expected.
    let query_target = if condition.contains("[?") && !context_json.is_array() {
        JsonValue::Array(vec![context_json.clone()])
    } else {
        context_json.clone()
    };

    // Parse: $.path == value or $.path != value (but not inside filter expressions)
    // Only split on == or != if they're not inside a filter [?...]
    let is_filter = condition.starts_with("$[?");
    if !is_filter {
        let ops = ["==", "!="];
        for op in ops {
            if let Some((path, expected)) = condition.split_once(op) {
                let path = path.trim();
                let expected = expected.trim();

                let jsonpath = match JsonPath::parse(path) {
                    Ok(p) => p,
                    Err(_) => return false,
                };

                let nodes: Vec<_> = jsonpath.query(&query_target).all();
                if nodes.is_empty() {
                    return false;
                }

                let actual = nodes[0];
                let expected_val = parse_literal(expected);
                return compare_values(actual, &expected_val, op);
            }
        }
    }

    // Filter expression or existence check
    let jsonpath = match JsonPath::parse(condition) {
        Ok(p) => p,
        Err(_) => return false,
    };
    !jsonpath.query(&query_target).all().is_empty()
}

fn evaluate_regex(c: &Criterion, resp: &ResponseContext<'_>) -> bool {
    let context_expr = match &c.context {
        Some(ctx) => ctx.as_str(),
        None => return false,
    };

    let context_json = resolve_runtime_expr(context_expr, resp);
    let context_str = match context_json {
        JsonValue::String(s) => s,
        v => v.to_string(),
    };

    let pattern = c.condition.trim();
    Regex::new(pattern).map(|re| re.is_match(&context_str)).unwrap_or(false)
}

/// Resolve an Arazzo runtime expression to a JSON value (sync, for criteria evaluation)
fn resolve_runtime_expr(expr: &str, resp: &ResponseContext<'_>) -> JsonValue {
    let parsed = match parse_runtime_expr(expr.trim()) {
        Ok(p) => p,
        Err(_) => return JsonValue::Null,
    };

    match parsed {
        RuntimeExpr::StatusCode => JsonValue::Number(resp.status.into()),
        RuntimeExpr::Response(source) => match source {
            Source::Header(h) => {
                let v = resp.headers.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&h))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                JsonValue::String(v)
            }
            Source::Body { pointer } => {
                let json = match &resp.body_json {
                    Some(j) => j.clone(),
                    None => return JsonValue::Null,
                };
                match pointer {
                    Some(ptr) => json.pointer(ptr.as_str()).cloned().unwrap_or(JsonValue::Null),
                    None => json,
                }
            }
            _ => JsonValue::Null,
        },
        _ => JsonValue::Null,
    }
}

fn parse_literal(s: &str) -> JsonValue {
    let s = s.trim();

    // Try JSON first
    if let Ok(v) = serde_json::from_str::<JsonValue>(s) {
        return v;
    }

    // Quoted string
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return JsonValue::String(s[1..s.len() - 1].to_string());
    }

    // Boolean
    if s == "true" { return JsonValue::Bool(true); }
    if s == "false" { return JsonValue::Bool(false); }
    if s == "null" { return JsonValue::Null; }

    // Number
    if let Ok(n) = s.parse::<i64>() {
        return JsonValue::Number(n.into());
    }
    if let Ok(n) = s.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return JsonValue::Number(num);
        }
    }

    JsonValue::String(s.to_string())
}

fn compare_values(actual: &JsonValue, expected: &JsonValue, op: &str) -> bool {
    match op {
        "==" => json_eq(actual, expected),
        "!=" => !json_eq(actual, expected),
        "<" => json_cmp(actual, expected).map(|o| o.is_lt()).unwrap_or(false),
        ">" => json_cmp(actual, expected).map(|o| o.is_gt()).unwrap_or(false),
        "<=" => json_cmp(actual, expected).map(|o| o.is_le()).unwrap_or(false),
        ">=" => json_cmp(actual, expected).map(|o| o.is_ge()).unwrap_or(false),
        _ => false,
    }
}

fn json_eq(a: &JsonValue, b: &JsonValue) -> bool {
    match (a, b) {
        (JsonValue::Null, JsonValue::Null) => true,
        (JsonValue::Bool(a), JsonValue::Bool(b)) => a == b,
        (JsonValue::Number(a), JsonValue::Number(b)) => a.as_f64() == b.as_f64(),
        (JsonValue::String(a), JsonValue::String(b)) => a == b,
        (JsonValue::Array(a), JsonValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| json_eq(x, y))
        }
        (JsonValue::Object(a), JsonValue::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).map(|bv| json_eq(v, bv)).unwrap_or(false))
        }
        _ => false,
    }
}

fn json_cmp(a: &JsonValue, b: &JsonValue) -> Option<std::cmp::Ordering> {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => a.partial_cmp(&b),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_resp(status: u16, body: &str) -> ResponseContext<'static> {
        let body_bytes: &'static [u8] = Box::leak(body.as_bytes().to_vec().into_boxed_slice());
        let headers: &'static BTreeMap<String, String> = Box::leak(Box::new(BTreeMap::new()));
        ResponseContext {
            status,
            headers,
            body: body_bytes,
            body_json: serde_json::from_str(body).ok(),
        }
    }

    #[test]
    fn test_simple_status_code() {
        let resp = make_resp(200, "{}");
        let c = Criterion {
            context: None,
            condition: "$statusCode == 200".to_string(),
            r#type: None,
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp));
    }

    #[test]
    fn test_jsonpath_boolean() {
        let resp = make_resp(200, r#"{"authenticated": true}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "$.authenticated == true".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp));
    }

    #[test]
    fn test_jsonpath_string() {
        let resp = make_resp(200, r#"{"user": "testuser"}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: r#"$.user == "testuser""#.to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp));
    }

    #[test]
    fn test_regex() {
        let resp = make_resp(200, r#""hello world""#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "hello.*".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Regex)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp));
    }

    #[test]
    fn test_jsonpath_filter_existence() {
        let resp = make_resp(200, r#"{"origin": "1.2.3.4"}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "$[?(@.origin)]".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp), "filter existence check should pass");
    }

    #[test]
    fn test_jsonpath_filter_with_comparison() {
        let resp = make_resp(200, r#"{"authenticated": true, "user": "test"}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "$[?(@.authenticated == true)]".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp), "filter comparison should pass");
    }

    #[test]
    fn test_jsonpath_filter_negative() {
        let resp = make_resp(200, r#"{"other": "value"}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "$[?(@.origin)]".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(!evaluate_criterion(&c, &resp), "filter should fail when field missing");
    }

    #[test]
    fn test_jsonpath_bracket_notation() {
        let resp = make_resp(200, r#"{"user-agent": "test-agent"}"#);
        let c = Criterion {
            context: Some("$response.body".to_string()),
            condition: "$[?(@['user-agent'])]".to_string(),
            r#type: Some(CriterionType::Known(KnownCriterionType::Jsonpath)),
            extensions: Default::default(),
        };
        assert!(evaluate_criterion(&c, &resp), "bracket notation should work");
    }
}

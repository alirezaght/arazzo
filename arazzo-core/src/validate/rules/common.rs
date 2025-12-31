use crate::expressions::{parse_runtime_expr, parse_template, validate_value_expressions};
use crate::validate::validator::{Validator, MAP_KEY_RE};

pub(crate) fn validate_map_keys<'a>(
    v: &mut Validator,
    path: &str,
    keys: impl Iterator<Item = &'a String>,
) {
    for key in keys {
        if !MAP_KEY_RE.is_match(key) {
            v.push(
                format!("{path}.{key}"),
                "map key must match regex ^[a-zA-Z0-9\\.\\-_]+$",
            );
        }
    }
}

pub(crate) fn validate_runtime_expr(v: &mut Validator, path: &str, expr: &str) {
    if let Err(e) = parse_runtime_expr(expr) {
        v.push(path, format!("invalid runtime expression: {e}"));
    }
}

pub(crate) fn validate_template_string(v: &mut Validator, path: &str, s: &str) {
    if let Err(e) = parse_template(s) {
        v.push(path, format!("invalid template expression: {e}"));
    }
}

pub(crate) fn validate_value_exprs(v: &mut Validator, path: &str, value: &serde_json::Value) {
    if let Err(e) = validate_value_expressions(value) {
        v.push(path, format!("invalid expression inside value: {e}"));
    }
}

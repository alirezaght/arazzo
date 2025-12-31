use super::runtime::{parse_runtime_expr, RuntimeExprError};
use crate::types::AnyValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Expr(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub segments: Vec<Segment>,
}

pub fn parse_template(input: &str) -> Result<Template, TemplateError> {
    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Only treat `{ ... }` as an embedded expression if it looks like `{ $... }`.
            // Otherwise, keep scanning; this avoids swallowing JSON objects in templated payload strings.
            let mut lookahead = chars.clone();
            while let Some(ws) = lookahead.peek() {
                if ws.is_whitespace() {
                    lookahead.next();
                } else {
                    break;
                }
            }
            let is_expr = matches!(lookahead.peek(), Some('$'));
            if !is_expr {
                buf.push('{');
                continue;
            }

            // Find matching } (no nesting support).
            let mut inner = String::new();
            let mut found = false;
            for n in chars.by_ref() {
                if n == '}' {
                    found = true;
                    break;
                }
                inner.push(n);
            }

            if !found {
                // Unclosed expression-like start; treat as a hard error.
                return Err(TemplateError::UnclosedExpression);
            }

            let inner_trimmed = inner.trim();
            // At this point, it should start with '$' due to lookahead.
            parse_runtime_expr(inner_trimmed).map_err(TemplateError::InvalidRuntimeExpr)?;
            if !buf.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut buf)));
            }
            segments.push(Segment::Expr(inner_trimmed.to_string()));
        } else {
            buf.push(ch);
        }
    }

    if !buf.is_empty() {
        segments.push(Segment::Literal(buf));
    }

    Ok(Template { segments })
}

pub fn validate_value_expressions(value: &AnyValue) -> Result<(), TemplateError> {
    match value {
        AnyValue::Null | AnyValue::Bool(_) | AnyValue::Number(_) => Ok(()),
        AnyValue::String(s) => validate_string_expressions(s),
        AnyValue::Array(arr) => {
            for v in arr {
                validate_value_expressions(v)?;
            }
            Ok(())
        }
        AnyValue::Object(map) => {
            for (_k, v) in map {
                validate_value_expressions(v)?;
            }
            Ok(())
        }
    }
}

fn validate_string_expressions(s: &str) -> Result<(), TemplateError> {
    let trimmed = s.trim();
    if trimmed.starts_with('$') {
        parse_runtime_expr(trimmed).map_err(TemplateError::InvalidRuntimeExpr)?;
        return Ok(());
    }

    // Validate embedded expressions in templates.
    let _ = parse_template(s)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("invalid runtime expression: {0}")]
    InvalidRuntimeExpr(#[from] RuntimeExprError),
    #[error("unclosed embedded expression (missing '}}')")]
    UnclosedExpression,
}

use std::sync::LazyLock;

use regex::Regex;

use super::json_pointer::{JsonPointer, JsonPointerError};

static TCHAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$").expect("valid regex"));

static NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9\.\-_]+$").expect("valid regex"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeExpr {
    Url,
    Method,
    StatusCode,
    Request(Source),
    Response(Source),
    Inputs(NamePath),
    Outputs(NamePath),
    Steps(NamePath),
    Workflows(NamePath),
    SourceDescriptions(NamePath),
    Components(NamePath),
    ComponentsParameters(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Header(String),
    Query(String),
    Path(String),
    Body { pointer: Option<JsonPointer> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePath {
    pub root: String,
    pub rest: Vec<String>,
    pub pointer: Option<JsonPointer>,
}

pub fn parse_runtime_expr(input: &str) -> Result<RuntimeExpr, RuntimeExprError> {
    let s = input.trim();
    if !s.starts_with('$') {
        return Err(RuntimeExprError::MissingDollarPrefix);
    }

    // Split optional `#<json-pointer>` suffix.
    let (head, pointer) = split_pointer_suffix(&s[1..])?;

    if head == "url" {
        return Ok(RuntimeExpr::Url);
    }
    if head == "method" {
        return Ok(RuntimeExpr::Method);
    }
    if head == "statusCode" {
        return Ok(RuntimeExpr::StatusCode);
    }

    if let Some(rest) = head.strip_prefix("request.") {
        return Ok(RuntimeExpr::Request(parse_source(rest)?));
    }
    if let Some(rest) = head.strip_prefix("response.") {
        return Ok(RuntimeExpr::Response(parse_source(rest)?));
    }
    if let Some(rest) = head.strip_prefix("inputs.") {
        return Ok(RuntimeExpr::Inputs(parse_name_path(rest, pointer)?));
    }
    if let Some(rest) = head.strip_prefix("outputs.") {
        return Ok(RuntimeExpr::Outputs(parse_name_path(rest, pointer)?));
    }
    if let Some(rest) = head.strip_prefix("steps.") {
        return Ok(RuntimeExpr::Steps(parse_name_path(rest, pointer)?));
    }
    if let Some(rest) = head.strip_prefix("workflows.") {
        return Ok(RuntimeExpr::Workflows(parse_name_path(rest, pointer)?));
    }
    if let Some(rest) = head.strip_prefix("sourceDescriptions.") {
        return Ok(RuntimeExpr::SourceDescriptions(parse_name_path(
            rest, pointer,
        )?));
    }

    if let Some(rest) = head.strip_prefix("components.parameters.") {
        let name = rest;
        if name.is_empty() {
            return Err(RuntimeExprError::EmptyName);
        }
        if !NAME_RE.is_match(name) {
            return Err(RuntimeExprError::InvalidName(name.to_string()));
        }
        if pointer.is_some() {
            return Err(RuntimeExprError::PointerNotAllowed);
        }
        return Ok(RuntimeExpr::ComponentsParameters(name.to_string()));
    }

    if let Some(rest) = head.strip_prefix("components.") {
        return Ok(RuntimeExpr::Components(parse_name_path(rest, pointer)?));
    }

    Err(RuntimeExprError::UnknownExpression(head.to_string()))
}

fn split_pointer_suffix(s: &str) -> Result<(String, Option<JsonPointer>), RuntimeExprError> {
    if let Some((head, frag)) = s.split_once('#') {
        let ptr = JsonPointer::parse(frag).map_err(RuntimeExprError::InvalidJsonPointer)?;
        Ok((head.to_string(), Some(ptr)))
    } else {
        Ok((s.to_string(), None))
    }
}

fn parse_source(rest: &str) -> Result<Source, RuntimeExprError> {
    if let Some(token) = rest.strip_prefix("header.") {
        if token.is_empty() {
            return Err(RuntimeExprError::EmptyName);
        }
        if !TCHAR_RE.is_match(token) {
            return Err(RuntimeExprError::InvalidHeaderToken(token.to_string()));
        }
        return Ok(Source::Header(token.to_string()));
    }
    if let Some(name) = rest.strip_prefix("query.") {
        validate_name(name)?;
        return Ok(Source::Query(name.to_string()));
    }
    if let Some(name) = rest.strip_prefix("path.") {
        validate_name(name)?;
        return Ok(Source::Path(name.to_string()));
    }
    if rest == "body" {
        return Ok(Source::Body { pointer: None });
    }
    if let Some(ptr) = rest.strip_prefix("body#") {
        let pointer = JsonPointer::parse(ptr).map_err(RuntimeExprError::InvalidJsonPointer)?;
        return Ok(Source::Body {
            pointer: Some(pointer),
        });
    }

    Err(RuntimeExprError::InvalidSource(rest.to_string()))
}

fn parse_name_path(rest: &str, pointer: Option<JsonPointer>) -> Result<NamePath, RuntimeExprError> {
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.is_empty() {
        return Err(RuntimeExprError::EmptyName);
    }
    if parts.iter().any(|p| p.is_empty()) {
        return Err(RuntimeExprError::EmptyName);
    }

    let root = parts[0].to_string();
    validate_name(&root)?;

    let mut remaining = Vec::new();
    for p in parts.iter().skip(1) {
        validate_name(p)?;
        remaining.push((*p).to_string());
    }

    Ok(NamePath {
        root,
        rest: remaining,
        pointer,
    })
}

fn validate_name(name: &str) -> Result<(), RuntimeExprError> {
    if name.is_empty() {
        return Err(RuntimeExprError::EmptyName);
    }
    if !NAME_RE.is_match(name) {
        return Err(RuntimeExprError::InvalidName(name.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeExprError {
    #[error("runtime expression must start with '$'")]
    MissingDollarPrefix,
    #[error("unknown runtime expression: {0}")]
    UnknownExpression(String),
    #[error("invalid source reference: {0}")]
    InvalidSource(String),
    #[error("name segment must not be empty")]
    EmptyName,
    #[error("invalid name segment: {0}")]
    InvalidName(String),
    #[error("invalid header token: {0}")]
    InvalidHeaderToken(String),
    #[error("invalid json pointer: {0}")]
    InvalidJsonPointer(#[from] JsonPointerError),
    #[error("json pointer is not allowed on this runtime expression")]
    PointerNotAllowed,
}

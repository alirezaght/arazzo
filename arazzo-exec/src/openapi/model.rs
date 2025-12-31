use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenApiDoc {
    /// Original location (URL or file path) used to load.
    pub source_url: String,
    /// Parsed OpenAPI document as JSON value (works for both JSON and YAML inputs).
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenApiDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedOperation {
    pub source_name: String,
    /// Base URL selected from OpenAPI `servers` (operation/path-item/doc), if present.
    pub base_url: String,
    pub method: String,
    pub path: String,
    pub operation_id: Option<String>,
    pub shape: CompiledOperationShape,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompiledOperationShape {
    pub parameters: Vec<OpenApiParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body_content_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenApiParam {
    pub name: String,
    pub location: OpenApiParamLocation,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenApiParamLocation {
    Path,
    Query,
    Header,
    Cookie,
}

pub(crate) fn location_from_str(s: &str) -> Option<OpenApiParamLocation> {
    match s {
        "path" => Some(OpenApiParamLocation::Path),
        "query" => Some(OpenApiParamLocation::Query),
        "header" => Some(OpenApiParamLocation::Header),
        "cookie" => Some(OpenApiParamLocation::Cookie),
        _ => None,
    }
}

pub(crate) fn method_keys() -> &'static [&'static str] {
    &[
        "get", "put", "post", "delete", "options", "head", "patch", "trace",
    ]
}

pub(crate) fn decode_json_pointer_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

pub(crate) fn pointer_from_str(pointer: &str) -> Option<String> {
    if pointer.is_empty() {
        return Some("".to_string());
    }
    if pointer.starts_with('/') {
        Some(pointer.to_string())
    } else if pointer.starts_with('#') {
        Some(pointer.trim_start_matches('#').to_string())
    } else {
        None
    }
}

pub(crate) fn collect_content_types(request_body: &serde_json::Value) -> Option<Vec<String>> {
    let content = request_body.get("content")?.as_object()?;
    let mut keys: Vec<String> = content.keys().cloned().collect();
    keys.sort();
    Some(keys)
}

pub(crate) fn is_request_body_required(request_body: &serde_json::Value) -> Option<bool> {
    request_body.get("required").and_then(|v| v.as_bool())
}

pub(crate) fn extract_parameter_obj(p: &serde_json::Value) -> Option<OpenApiParam> {
    if p.get("$ref").is_some() {
        return None;
    }
    let name = p.get("name").and_then(|v| v.as_str())?;
    let loc = p
        .get("in")
        .and_then(|v| v.as_str())
        .and_then(location_from_str)?;
    let mut required = p.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
    if loc == OpenApiParamLocation::Path {
        required = true;
    }
    Some(OpenApiParam {
        name: name.to_string(),
        location: loc,
        required,
    })
}

pub(crate) fn dedupe_params(params: Vec<OpenApiParam>) -> Vec<OpenApiParam> {
    let mut map: BTreeMap<(OpenApiParamLocation, String), bool> = BTreeMap::new();
    for p in &params {
        map.entry((p.location, p.name.clone()))
            .and_modify(|req| *req = *req || p.required)
            .or_insert(p.required);
    }
    let mut out = map
        .into_iter()
        .map(|((loc, name), required)| OpenApiParam {
            name,
            location: loc,
            required,
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        (location_rank(a.location), &a.name).cmp(&(location_rank(b.location), &b.name))
    });
    out
}

fn location_rank(loc: OpenApiParamLocation) -> u8 {
    match loc {
        OpenApiParamLocation::Path => 0,
        OpenApiParamLocation::Query => 1,
        OpenApiParamLocation::Header => 2,
        OpenApiParamLocation::Cookie => 3,
    }
}


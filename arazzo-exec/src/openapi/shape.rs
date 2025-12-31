use std::collections::HashSet;

use crate::openapi::model::{
    collect_content_types, extract_parameter_obj, is_request_body_required, CompiledOperationShape,
};
use crate::openapi::refs::resolve_ref;

pub(crate) fn compile_operation_shape(
    doc: &serde_json::Value,
    source_name: &str,
    path: &str,
    method: &str,
    operation: &serde_json::Value,
) -> (CompiledOperationShape, Vec<String>) {
    let mut diagnostics = Vec::<String>::new();

    // Merge path-level parameters and operation-level parameters.
    let mut params = Vec::new();
    if let Some(path_item) = doc
        .get("paths")
        .and_then(|p| p.get(path))
        .and_then(|v| v.as_object())
    {
        if let Some(p) = path_item.get("parameters") {
            params.extend(extract_params_with_refs(
                doc,
                source_name,
                "pathItem.parameters",
                p,
                &mut diagnostics,
            ));
        }
        if let Some(op_obj) = path_item
            .get(method)
            .or_else(|| path_item.get(&method.to_lowercase()))
        {
            // optional: op-specific inside path item already provided via `operation`.
            let _ = op_obj;
        }
    }

    if let Some(p) = operation.get("parameters") {
        params.extend(extract_params_with_refs(
            doc,
            source_name,
            "operation.parameters",
            p,
            &mut diagnostics,
        ));
    }
    let params = crate::openapi::model::dedupe_params(params);

    let request_body = operation.get("requestBody");
    let (rb_required, rb_cts) = match request_body {
        None => (None, None),
        Some(rb) => {
            let mut visited = HashSet::new();
            let rb_resolved = if let Some(r) = rb.get("$ref").and_then(|v| v.as_str()) {
                match resolve_ref(doc, r, &mut visited) {
                    Ok(v) => v,
                    Err(e) => {
                        diagnostics.push(format!("{source_name}: requestBody {e}"));
                        rb
                    }
                }
            } else {
                rb
            };

            (
                is_request_body_required(rb_resolved),
                collect_content_types(rb_resolved),
            )
        }
    };

    let _ = method;
    (
        CompiledOperationShape {
            parameters: params,
            request_body_required: rb_required,
            request_body_content_types: rb_cts,
        },
        diagnostics,
    )
}

pub(crate) fn select_base_url(
    doc: &serde_json::Value,
    path: &str,
    method: &str,
    operation: &serde_json::Value,
) -> Option<String> {
    // Prefer operation.servers[0].url, then path-item.servers[0].url, then doc.servers[0].url.
    if let Some(url) = servers_first_url(operation) {
        return Some(url);
    }
    if let Some(path_item) = doc.get("paths").and_then(|p| p.get(path)) {
        if let Some(url) = servers_first_url(path_item) {
            return Some(url);
        }
    }
    let _ = method;
    servers_first_url(doc)
}

fn servers_first_url(v: &serde_json::Value) -> Option<String> {
    let servers = v.get("servers")?.as_array()?;
    let first = servers.first()?.as_object()?;
    first.get("url")?.as_str().map(|s| s.to_string())
}

fn extract_params_with_refs(
    doc: &serde_json::Value,
    source_name: &str,
    ctx: &str,
    parameters: &serde_json::Value,
    diagnostics: &mut Vec<String>,
) -> Vec<crate::openapi::model::OpenApiParam> {
    let mut out = Vec::new();
    let Some(arr) = parameters.as_array() else {
        return out;
    };

    for p in arr {
        if let Some(r) = p.get("$ref").and_then(|v| v.as_str()) {
            let mut visited = HashSet::new();
            match resolve_ref(doc, r, &mut visited) {
                Ok(v) => {
                    if let Some(param) = extract_parameter_obj(v) {
                        out.push(param);
                    } else {
                        diagnostics.push(format!(
                            "{source_name}: {ctx} $ref did not resolve to a Parameter Object"
                        ));
                    }
                }
                Err(e) => diagnostics.push(format!("{source_name}: {ctx} {e}")),
            }
        } else if let Some(param) = extract_parameter_obj(p) {
            out.push(param);
        }
    }

    out
}

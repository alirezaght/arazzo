use std::collections::BTreeSet;

use arazzo_core::expressions::{RuntimeExpr, parse_runtime_expr};
use arazzo_core::types::Workflow;

use crate::openapi::model::{ResolvedOperation, method_keys};
use crate::openapi::shape::{compile_operation_shape, select_base_url};

pub(crate) enum OperationIdSelection {
    Selected {
        source_name: String,
        operation_id: String,
        warnings: Vec<String>,
    },
    Error(String),
}

pub(crate) fn select_source_for_operation_id(
    sources: &crate::openapi::resolver::ResolvedSources,
    _workflow: &Workflow,
    operation_id_raw: &str,
) -> OperationIdSelection {
    let trimmed = operation_id_raw.trim();
    if trimmed.starts_with('$') {
        let expr = match parse_runtime_expr(trimmed) {
            Ok(e) => e,
            Err(e) => return OperationIdSelection::Error(format!("invalid operationId runtime expression: {e}")),
        };
        if let RuntimeExpr::SourceDescriptions(np) = expr {
            // expected: $sourceDescriptions.<name>.<operationId>
            let Some(op_id) = np.rest.first() else {
                return OperationIdSelection::Error("qualified operationId must include the operationId segment".to_string());
            };
            return OperationIdSelection::Selected {
                source_name: np.root,
                operation_id: op_id.clone(),
                warnings: Vec::new(),
            };
        }
        return OperationIdSelection::Error(
            "operationId runtime expression must be $sourceDescriptions.<name>.<operationId>".to_string(),
        );
    }

    if sources.openapi_docs.is_empty() {
        return OperationIdSelection::Error("no OpenAPI sources available".to_string());
    }

    if sources.openapi_docs.len() == 1 {
        let source_name = sources
            .openapi_docs
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| {
                panic!("expected exactly one OpenAPI source, but found none. This is a bug - please report it.");
            });
        return OperationIdSelection::Selected {
            source_name,
            operation_id: trimmed.to_string(),
            warnings: Vec::new(),
        };
    }

    // Multiple sources: search for matches and decide deterministically.
    let mut matched_sources = BTreeSet::<String>::new();
    for (name, doc) in &sources.openapi_docs {
        if operation_id_exists(&doc.raw, trimmed) {
            matched_sources.insert(name.clone());
        }
    }

    match matched_sources.len() {
        0 => OperationIdSelection::Error(format!(
            "operationId '{trimmed}' not found in any OpenAPI source (available: {})",
            sources.openapi_docs.keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        1 => {
            let source_name = matched_sources.iter().next().cloned().unwrap_or_else(|| {
                panic!("expected exactly one matched source, but found none. This is a bug - please report it.");
            });
            OperationIdSelection::Selected {
                source_name: source_name.clone(),
                operation_id: trimmed.to_string(),
                warnings: vec![format!(
                    "unqualified operationId '{trimmed}' resolved to source '{source_name}' (consider qualifying with $sourceDescriptions.{source_name}.{trimmed})"
                )],
            }
        }
        _ => OperationIdSelection::Error(format!(
            "ambiguous operationId '{trimmed}' found in sources: {} (must qualify with $sourceDescriptions.<name>.<operationId>)",
            matched_sources.into_iter().collect::<Vec<_>>().join(", ")
        )),
    }
}

fn operation_id_exists(doc: &serde_json::Value, operation_id: &str) -> bool {
    let Some(paths) = doc.get("paths").and_then(|v| v.as_object()) else {
        return false;
    };
    for (_path, item) in paths {
        let Some(item_obj) = item.as_object() else {
            continue;
        };
        for method in method_keys() {
            let Some(op) = item_obj.get(*method) else {
                continue;
            };
            let Some(op_obj) = op.as_object() else {
                continue;
            };
            let Some(opid) = op_obj.get("operationId").and_then(|v| v.as_str()) else {
                continue;
            };
            if opid == operation_id {
                return true;
            }
        }
    }
    false
}

pub(crate) fn find_operation_by_id(
    doc: &serde_json::Value,
    source_name: &str,
    operation_id: &str,
) -> Option<(ResolvedOperation, Vec<String>)> {
    let paths = doc.get("paths")?.as_object()?;
    for (path, item) in paths {
        let item_obj = item.as_object()?;
        for method in method_keys() {
            let Some(op) = item_obj.get(*method) else {
                continue;
            };
            let op_obj = op.as_object()?;
            let Some(opid) = op_obj.get("operationId").and_then(|v| v.as_str()) else {
                continue;
            };
            if opid == operation_id {
                let base_url = select_base_url(doc, path, method, op).unwrap_or_default();
                let (shape, diag) = compile_operation_shape(doc, source_name, path, method, op);
                return Some((
                    ResolvedOperation {
                        source_name: source_name.to_string(),
                        base_url,
                        method: method.to_uppercase(),
                        path: path.clone(),
                        operation_id: Some(operation_id.to_string()),
                        shape,
                    },
                    diag,
                ));
            }
        }
    }
    None
}


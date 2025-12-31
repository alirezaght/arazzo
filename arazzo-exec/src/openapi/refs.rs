use std::collections::HashSet;

pub(crate) fn resolve_ref<'a>(
    doc: &'a serde_json::Value,
    ref_str: &str,
    visited: &mut HashSet<String>,
) -> Result<&'a serde_json::Value, RefError> {
    // Only support local refs for now: "#/..."
    if !ref_str.starts_with('#') {
        return Err(RefError::ExternalRef(ref_str.to_string()));
    }

    let pointer = ref_str.trim_start_matches('#');
    if !visited.insert(ref_str.to_string()) {
        return Err(RefError::Cycle(ref_str.to_string()));
    }

    doc.pointer(pointer)
        .ok_or_else(|| RefError::NotFound(ref_str.to_string()))
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RefError {
    #[error("unsupported external $ref: {0}")]
    ExternalRef(String),
    #[error("unresolvable $ref: {0}")]
    NotFound(String),
    #[error("cyclic $ref: {0}")]
    Cycle(String),
}

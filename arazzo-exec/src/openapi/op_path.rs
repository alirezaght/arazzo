use crate::openapi::model::{decode_json_pointer_token, pointer_from_str};

pub fn parse_operation_path_ref(op_path: &str) -> Result<(String, String, String, String), String> {
    // Expected example:
    // '{$sourceDescriptions.petStoreDescription.url}#/paths/~1pet~1findByStatus/get'
    let (before_hash, after_hash) = op_path
        .split_once('#')
        .ok_or_else(|| "operationPath must include a '#/paths/..' JSON pointer".to_string())?;

    // find the {$sourceDescriptions.<name>.url} segment (best-effort)
    let src_name = extract_source_name_from_template(before_hash)
        .ok_or_else(|| "operationPath must contain {$sourceDescriptions.<name>.url}".to_string())?;

    let pointer = pointer_from_str(after_hash)
        .ok_or_else(|| "invalid JSON pointer fragment in operationPath".to_string())?;

    // Derive method + path from pointer: /paths/<encoded-path>/<method>
    let parts: Vec<&str> = pointer.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 3 || parts[0] != "paths" {
        return Err("operationPath pointer must point under /paths/<path>/<method>".to_string());
    }
    let path = decode_json_pointer_token(parts[1]);
    let method = parts[2].to_string();
    Ok((src_name, pointer, method, path))
}

fn extract_source_name_from_template(s: &str) -> Option<String> {
    // Look for {$sourceDescriptions.<name>.url}
    let open = s.find("{$sourceDescriptions.")?;
    let sub = &s[open + "{$sourceDescriptions.".len()..];
    let (name, rest) = sub.split_once('.')?;
    if !rest.starts_with("url}") {
        return None;
    }
    Some(name.to_string())
}


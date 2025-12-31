use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct SensitiveHeadersConfig {
    /// Lowercased header names that must always be redacted.
    pub always_redact: Vec<String>,
}

impl Default for SensitiveHeadersConfig {
    fn default() -> Self {
        Self {
            always_redact: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SanitizedHeaders {
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SanitizedBody {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

pub(crate) fn sanitize_headers(
    headers: &BTreeMap<String, String>,
    sensitive: &SensitiveHeadersConfig,
    secret_derived_headers: &[String],
) -> SanitizedHeaders {
    let mut out = headers.clone();
    for name in sensitive
        .always_redact
        .iter()
        .chain(secret_derived_headers.iter())
    {
        redact_case_insensitive(&mut out, name);
    }
    SanitizedHeaders { headers: out }
}

pub(crate) fn truncate_body(body: &[u8], max_bytes: usize) -> SanitizedBody {
    if body.len() <= max_bytes {
        SanitizedBody {
            bytes: body.to_vec(),
            truncated: false,
        }
    } else {
        SanitizedBody {
            bytes: body[..max_bytes].to_vec(),
            truncated: true,
        }
    }
}

pub(crate) fn redact_body_with_secrets(body: &[u8], max_bytes: usize) -> SanitizedBody {
    const REDACTED: &[u8] = b"<body-redacted:contains-secrets>";
    let len = REDACTED.len().min(max_bytes);
    SanitizedBody {
        bytes: REDACTED[..len].to_vec(),
        truncated: body.len() > max_bytes,
    }
}

fn redact_case_insensitive(map: &mut BTreeMap<String, String>, header_lower: &str) {
    let keys = map
        .keys()
        .filter(|k| k.eq_ignore_ascii_case(header_lower))
        .cloned()
        .collect::<Vec<_>>();
    for k in keys {
        map.insert(k, "<redacted>".to_string());
    }
}

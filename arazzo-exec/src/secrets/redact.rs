use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct RedactionPolicy {
    pub redact_authorization: bool,
    pub redact_cookie: bool,
    pub redact_set_cookie: bool,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            redact_authorization: true,
            redact_cookie: true,
            redact_set_cookie: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RedactedHeaders {
    pub headers: BTreeMap<String, String>,
}

pub fn redact_headers(
    headers: &BTreeMap<String, String>,
    policy: &RedactionPolicy,
    secret_derived_header_names: &[String],
) -> RedactedHeaders {
    let mut out = headers.clone();

    if policy.redact_authorization {
        remove_case_insensitive(&mut out, "authorization", "<redacted>");
    }
    if policy.redact_cookie {
        remove_case_insensitive(&mut out, "cookie", "<redacted>");
    }
    if policy.redact_set_cookie {
        remove_case_insensitive(&mut out, "set-cookie", "<redacted>");
    }

    for name in secret_derived_header_names {
        remove_case_insensitive(&mut out, name, "<redacted>");
    }

    RedactedHeaders { headers: out }
}

fn remove_case_insensitive(map: &mut BTreeMap<String, String>, header: &str, replacement: &str) {
    // Find all keys that match case-insensitively and replace their values.
    let keys = map
        .keys()
        .filter(|k| k.eq_ignore_ascii_case(header))
        .cloned()
        .collect::<Vec<_>>();
    for k in keys {
        map.insert(k, replacement.to_string());
    }
}

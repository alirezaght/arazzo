#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer {
    raw: String,
}

impl JsonPointer {
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn parse(fragment: &str) -> Result<Self, JsonPointerError> {
        // Accept either "" (whole document) or a proper pointer "/a/b" or "#/a/b" style.
        // In Arazzo runtime expressions, we expect the `#` is handled outside and the pointer is the part after `#`.
        if fragment.is_empty() {
            return Ok(Self {
                raw: fragment.to_string(),
            });
        }
        if !fragment.starts_with('/') {
            return Err(JsonPointerError::InvalidPrefix);
        }

        // Validate escape sequences (RFC6901): "~0" and "~1" only.
        let mut chars = fragment.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '~' {
                match chars.next() {
                    Some('0' | '1') => {}
                    _ => return Err(JsonPointerError::InvalidEscape),
                }
            }
        }

        Ok(Self {
            raw: fragment.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JsonPointerError {
    #[error("json pointer must start with '/'")]
    InvalidPrefix,
    #[error("json pointer contains invalid escape (only ~0 and ~1 are allowed)")]
    InvalidEscape,
}

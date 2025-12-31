use std::sync::LazyLock;

use regex::Regex;

use crate::error::{ValidationError, Violation};
use crate::types::{ArazzoDocument, Extensions};

use super::rules;

pub(crate) static ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_\-]+$").expect("valid"));
pub(crate) static MAP_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9\.\-_]+$").expect("valid"));

pub struct Validator {
    violations: Vec<Violation>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    pub fn finish(self) -> Result<(), ValidationError> {
        if self.violations.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::new(self.violations))
        }
    }

    pub fn validate_document(&mut self, doc: &ArazzoDocument) {
        rules::document::validate_document(self, doc);
    }

    pub(crate) fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.violations.push(Violation::new(path, message));
    }

    pub(crate) fn validate_spec_version(&mut self, path: &str, version: &str) {
        // Spec says tooling should treat 1.0.0 and 1.0.1 as the same feature-set (major.minor).
        // We enforce that major.minor == 1.0.
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            self.push(path, "must be a semver-like string (major.minor[.patch])");
            return;
        }
        if parts[0] != "1" || parts[1] != "0" {
            self.push(path, "only Arazzo spec 1.0.x is currently supported");
        }
    }

    pub(crate) fn validate_extensions(&mut self, path: &str, ext: &Extensions) {
        for key in ext.keys() {
            if !key.starts_with("x-") {
                self.push(
                    format!("{path}.{key}"),
                    "unknown field (only x-* specification extensions are allowed)",
                );
            }
        }
    }
}


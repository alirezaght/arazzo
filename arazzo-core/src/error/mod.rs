use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArazzoError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("failed to parse as JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to parse as YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("unable to auto-detect document format (neither valid JSON nor valid YAML)")]
    UnknownFormat,
}

#[derive(Debug, Error)]
#[error("arazzo document failed validation ({violations_len} violations)")]
pub struct ValidationError {
    pub violations: Vec<Violation>,
    violations_len: usize,
}

impl ValidationError {
    pub fn new(violations: Vec<Violation>) -> Self {
        let violations_len = violations.len();
        Self {
            violations,
            violations_len,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub message: String,
}

impl Violation {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

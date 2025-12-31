use crate::types::{AnyValue, Extensions};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadReplacement {
    pub target: String,
    pub value: AnyValue,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RequestBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<AnyValue>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacements: Option<Vec<PayloadReplacement>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


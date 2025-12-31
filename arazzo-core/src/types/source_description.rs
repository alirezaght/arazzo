use crate::types::Extensions;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceDescriptionType {
    Openapi,
    Arazzo,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceDescription {
    pub name: String,
    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub source_type: Option<SourceDescriptionType>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


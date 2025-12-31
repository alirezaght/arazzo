use crate::types::{Components, Extensions, Info, SourceDescription, Workflow};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArazzoDocument {
    /// The Arazzo Specification version (e.g. "1.0.1").
    pub arazzo: String,

    pub info: Info,

    #[serde(rename = "sourceDescriptions")]
    pub source_descriptions: Vec<SourceDescription>,

    pub workflows: Vec<Workflow>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


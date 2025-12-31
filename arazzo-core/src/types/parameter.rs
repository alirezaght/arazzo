use crate::types::{AnyValue, Extensions, ReusableObject};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<ParameterLocation>,

    pub value: AnyValue,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ParameterOrReusable {
    Parameter(Parameter),
    Reusable(ReusableObject),
}

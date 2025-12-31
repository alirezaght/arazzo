use crate::types::{Extensions, RuntimeExpression};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnownCriterionType {
    Simple,
    Regex,
    Jsonpath,
    Xpath,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CriterionExpressionLanguage {
    Jsonpath,
    Xpath,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CriterionExpressionType {
    pub r#type: CriterionExpressionLanguage,
    pub version: String,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum CriterionType {
    Known(KnownCriterionType),
    Custom(CriterionExpressionType),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Criterion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<RuntimeExpression>,

    pub condition: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CriterionType>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


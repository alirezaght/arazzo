use crate::types::{Criterion, Extensions, ReusableObject};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuccessActionType {
    End,
    Goto,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SuccessAction {
    pub name: String,

    #[serde(rename = "type")]
    pub action_type: SuccessActionType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stepId")]
    pub step_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<Criterion>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureActionType {
    End,
    Retry,
    Goto,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FailureAction {
    pub name: String,

    #[serde(rename = "type")]
    pub action_type: FailureActionType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stepId")]
    pub step_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryAfter")]
    pub retry_after_seconds: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryLimit")]
    pub retry_limit: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<Criterion>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum SuccessActionOrReusable {
    Action(SuccessAction),
    Reusable(ReusableObject),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum FailureActionOrReusable {
    Action(FailureAction),
    Reusable(ReusableObject),
}


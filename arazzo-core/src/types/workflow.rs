use std::collections::BTreeMap;

use crate::types::{
    Extensions, JsonSchema, ParameterOrReusable, RuntimeExpression, Step, FailureActionOrReusable,
    SuccessActionOrReusable,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Workflow {
    #[serde(rename = "workflowId")]
    pub workflow_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<JsonSchema>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dependsOn")]
    pub depends_on: Option<Vec<String>>,

    pub steps: Vec<Step>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "successActions")]
    pub success_actions: Option<Vec<SuccessActionOrReusable>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "failureActions")]
    pub failure_actions: Option<Vec<FailureActionOrReusable>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<BTreeMap<String, RuntimeExpression>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ParameterOrReusable>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


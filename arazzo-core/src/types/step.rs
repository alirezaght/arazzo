use std::collections::BTreeMap;

use crate::types::{
    Criterion, Extensions, FailureActionOrReusable, ParameterOrReusable, RequestBody,
    RuntimeExpression, SuccessActionOrReusable,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Step {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "stepId")]
    pub step_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "operationPath")]
    pub operation_path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ParameterOrReusable>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "successCriteria")]
    pub success_criteria: Option<Vec<Criterion>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "onSuccess")]
    pub on_success: Option<Vec<SuccessActionOrReusable>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "onFailure")]
    pub on_failure: Option<Vec<FailureActionOrReusable>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<BTreeMap<String, RuntimeExpression>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}

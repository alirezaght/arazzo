use std::collections::BTreeMap;

use crate::types::{Extensions, FailureAction, JsonSchema, Parameter, SuccessAction};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Components {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<BTreeMap<String, JsonSchema>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<BTreeMap<String, Parameter>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "successActions")]
    pub success_actions: Option<BTreeMap<String, SuccessAction>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "failureActions")]
    pub failure_actions: Option<BTreeMap<String, FailureAction>>,

    #[serde(flatten, default)]
    pub extensions: Extensions,
}


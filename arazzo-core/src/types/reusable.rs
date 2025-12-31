use crate::types::{AnyValue, RuntimeExpression};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReusableObject {
    pub reference: RuntimeExpression,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<AnyValue>,
}

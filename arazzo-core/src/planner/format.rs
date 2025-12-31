use crate::types::{FailureActionOrReusable, FailureActionType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanFormat {
    Text,
    Json,
    Dot,
}

pub(crate) fn format_failure_actions(actions: &[FailureActionOrReusable]) -> Vec<String> {
    actions
        .iter()
        .map(|a| match a {
            FailureActionOrReusable::Reusable(r) => format!("reusable: {}", r.reference),
            FailureActionOrReusable::Action(a) => match a.action_type {
                FailureActionType::End => format!("{}: end", a.name),
                FailureActionType::Goto => {
                    let target = a
                        .step_id
                        .as_ref()
                        .map(|s| format!("stepId={s}"))
                        .or_else(|| a.workflow_id.as_ref().map(|w| format!("workflowId={w}")))
                        .unwrap_or_else(|| "target=<missing>".to_string());
                    format!("{}: goto ({target})", a.name)
                }
                FailureActionType::Retry => {
                    let after = a.retry_after_seconds.map(|s| s.to_string()).unwrap_or_else(|| "default".into());
                    let limit = a.retry_limit.map(|n| n.to_string()).unwrap_or_else(|| "1".into());
                    let target = a
                        .step_id
                        .as_ref()
                        .map(|s| format!("stepId={s}"))
                        .or_else(|| a.workflow_id.as_ref().map(|w| format!("workflowId={w}")))
                        .unwrap_or_else(|| "current".to_string());
                    format!("{}: retry (after={after}s limit={limit} target={target})", a.name)
                }
            },
        })
        .collect()
}



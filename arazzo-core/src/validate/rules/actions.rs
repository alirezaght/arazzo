use std::collections::HashSet;

use crate::types::{
    FailureActionOrReusable, FailureActionType, SuccessActionOrReusable, SuccessActionType,
};
use crate::validate::rules::common::validate_runtime_expr;
use crate::validate::rules::criteria::validate_criteria_list;
use crate::validate::validator::Validator;

pub(crate) fn validate_success_action_list(
    v: &mut Validator,
    path: &str,
    actions: &[SuccessActionOrReusable],
    step_ids: Option<&HashSet<String>>,
) {
    let mut seen = HashSet::<String>::new();
    for (idx, item) in actions.iter().enumerate() {
        let ipath = format!("{path}[{idx}]");
        match item {
            SuccessActionOrReusable::Action(a) => {
                v.validate_extensions(&ipath, &a.extensions);
                if a.name.trim().is_empty() {
                    v.push(format!("{ipath}.name"), "must not be empty");
                }
                if !seen.insert(format!("name:{}", a.name)) {
                    v.push(ipath.as_str(), "duplicate success action name");
                }

                match a.action_type {
                    SuccessActionType::End => {
                        if a.workflow_id.is_some() || a.step_id.is_some() {
                            v.push(ipath.as_str(), "type=end must not specify workflowId or stepId");
                        }
                    }
                    SuccessActionType::Goto => {
                        let has_workflow = a.workflow_id.is_some();
                        let has_step = a.step_id.is_some();
                        if has_workflow == has_step {
                            v.push(
                                ipath.clone(),
                                "type=goto must specify exactly one of workflowId or stepId",
                            );
                        }
                        if let Some(workflow_id) = &a.workflow_id {
                            if workflow_id.trim().starts_with('$') {
                                validate_runtime_expr(v, &format!("{ipath}.workflowId"), workflow_id.trim());
                            }
                        }
                        if let (Some(step_id), Some(step_ids)) = (a.step_id.as_ref(), step_ids) {
                            if !step_ids.contains(step_id) {
                                v.push(
                                    format!("{ipath}.stepId"),
                                    "must reference a stepId in the current workflow",
                                );
                            }
                        }
                    }
                }

                if let Some(criteria) = &a.criteria {
                    validate_criteria_list(v, &format!("{ipath}.criteria"), criteria);
                }
            }
            SuccessActionOrReusable::Reusable(r) => {
                let key = format!("ref:{}", r.reference);
                if !seen.insert(key) {
                    v.push(ipath.as_str(), "duplicate reusable reference");
                }
                validate_runtime_expr(v, &format!("{ipath}.reference"), &r.reference);
                if !r.reference.starts_with("$components.successActions.") {
                    v.push(
                        format!("{ipath}.reference"),
                        "must reference $components.successActions.*",
                    );
                }
            }
        }
    }
}

pub(crate) fn validate_failure_action_list(
    v: &mut Validator,
    path: &str,
    actions: &[FailureActionOrReusable],
    step_ids: Option<&HashSet<String>>,
) {
    let mut seen = HashSet::<String>::new();
    for (idx, item) in actions.iter().enumerate() {
        let ipath = format!("{path}[{idx}]");
        match item {
            FailureActionOrReusable::Action(a) => {
                v.validate_extensions(&ipath, &a.extensions);
                if a.name.trim().is_empty() {
                    v.push(format!("{ipath}.name"), "must not be empty");
                }
                if !seen.insert(format!("name:{}", a.name)) {
                    v.push(ipath.clone(), "duplicate failure action name");
                }

                match a.action_type {
                    FailureActionType::End => {
                        if a.workflow_id.is_some()
                            || a.step_id.is_some()
                            || a.retry_after_seconds.is_some()
                            || a.retry_limit.is_some()
                        {
                            v.push(
                                ipath.clone(),
                                "type=end must not specify workflowId, stepId, retryAfter, or retryLimit",
                            );
                        }
                    }
                    FailureActionType::Goto => {
                        if a.retry_after_seconds.is_some() || a.retry_limit.is_some() {
                            v.push(ipath.clone(), "type=goto must not specify retryAfter or retryLimit");
                        }
                        let has_workflow = a.workflow_id.is_some();
                        let has_step = a.step_id.is_some();
                        if has_workflow == has_step {
                            v.push(
                                ipath.clone(),
                                "type=goto must specify exactly one of workflowId or stepId",
                            );
                        }
                        if let Some(workflow_id) = &a.workflow_id {
                            if workflow_id.trim().starts_with('$') {
                                validate_runtime_expr(v, &format!("{ipath}.workflowId"), workflow_id.trim());
                            }
                        }
                        if let (Some(step_id), Some(step_ids)) = (a.step_id.as_ref(), step_ids) {
                            if !step_ids.contains(step_id) {
                                v.push(
                                    format!("{ipath}.stepId"),
                                    "must reference a stepId in the current workflow",
                                );
                            }
                        }
                    }
                    FailureActionType::Retry => {
                        if let Some(secs) = a.retry_after_seconds {
                            if secs < 0.0 {
                                v.push(format!("{ipath}.retryAfter"), "must be non-negative");
                            }
                        }
                        let has_workflow = a.workflow_id.is_some();
                        let has_step = a.step_id.is_some();
                        if has_workflow && has_step {
                            v.push(
                                ipath.clone(),
                                "type=retry must not specify both workflowId and stepId",
                            );
                        }
                        if let Some(workflow_id) = &a.workflow_id {
                            if workflow_id.trim().starts_with('$') {
                                validate_runtime_expr(v, &format!("{ipath}.workflowId"), workflow_id.trim());
                            }
                        }
                        if let (Some(step_id), Some(step_ids)) = (a.step_id.as_ref(), step_ids) {
                            if !step_ids.contains(step_id) {
                                v.push(
                                    format!("{ipath}.stepId"),
                                    "must reference a stepId in the current workflow",
                                );
                            }
                        }
                    }
                }

                if let Some(criteria) = &a.criteria {
                    validate_criteria_list(v, &format!("{ipath}.criteria"), criteria);
                }
            }
            FailureActionOrReusable::Reusable(r) => {
                let key = format!("ref:{}", r.reference);
                if !seen.insert(key) {
                    v.push(ipath.clone(), "duplicate reusable reference");
                }
                validate_runtime_expr(v, &format!("{ipath}.reference"), &r.reference);
                if !r.reference.starts_with("$components.failureActions.") {
                    v.push(
                        format!("{ipath}.reference"),
                        "must reference $components.failureActions.*",
                    );
                }
            }
        }
    }
}


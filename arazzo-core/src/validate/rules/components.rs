use crate::types::Components;
use crate::validate::rules::common::{
    validate_map_keys, validate_runtime_expr, validate_value_exprs,
};
use crate::validate::rules::criteria::validate_criteria_list;
use crate::validate::validator::Validator;

pub(crate) fn validate_components(v: &mut Validator, components: &Components, path: &str) {
    v.validate_extensions(path, &components.extensions);

    if let Some(inputs) = &components.inputs {
        validate_map_keys(v, &format!("{path}.inputs"), inputs.keys());
    }
    if let Some(parameters) = &components.parameters {
        validate_map_keys(v, &format!("{path}.parameters"), parameters.keys());
        for (k, p) in parameters {
            let ppath = format!("{path}.parameters.{k}");
            v.validate_extensions(&ppath, &p.extensions);
            validate_value_exprs(v, &format!("{ppath}.value"), &p.value);
        }
    }
    if let Some(success_actions) = &components.success_actions {
        validate_map_keys(v, &format!("{path}.successActions"), success_actions.keys());
        for (k, a) in success_actions {
            let apath = format!("{path}.successActions.{k}");
            v.validate_extensions(&apath, &a.extensions);
            if a.action_type == crate::types::SuccessActionType::Goto {
                if let Some(workflow_id) = &a.workflow_id {
                    if workflow_id.trim().starts_with('$') {
                        validate_runtime_expr(
                            v,
                            &format!("{apath}.workflowId"),
                            workflow_id.trim(),
                        );
                    }
                }
            }
            if let Some(criteria) = &a.criteria {
                validate_criteria_list(v, &format!("{apath}.criteria"), criteria);
            }
        }
    }
    if let Some(failure_actions) = &components.failure_actions {
        validate_map_keys(v, &format!("{path}.failureActions"), failure_actions.keys());
        for (k, a) in failure_actions {
            let apath = format!("{path}.failureActions.{k}");
            v.validate_extensions(&apath, &a.extensions);
            if let Some(workflow_id) = &a.workflow_id {
                if workflow_id.trim().starts_with('$') {
                    validate_runtime_expr(v, &format!("{apath}.workflowId"), workflow_id.trim());
                }
            }
            if let Some(criteria) = &a.criteria {
                validate_criteria_list(v, &format!("{apath}.criteria"), criteria);
            }
        }
    }
}

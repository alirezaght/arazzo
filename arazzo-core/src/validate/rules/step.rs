use crate::types::Step;
use crate::validate::rules::{
    actions,
    common::{validate_map_keys, validate_runtime_expr, validate_template_string},
    criteria, parameters,
};
use crate::validate::validator::Validator;

pub(crate) fn validate_step(
    v: &mut Validator,
    step: &Step,
    path: &str,
    step_ids: &std::collections::HashSet<String>,
) {
    let op_fields = [
        step.operation_id.is_some(),
        step.operation_path.is_some(),
        step.workflow_id.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();

    if op_fields != 1 {
        v.push(
            path,
            "exactly one of operationId, operationPath, workflowId must be provided",
        );
    }

    if let Some(operation_path) = &step.operation_path {
        let op_path = format!("{path}.operationPath");
        if operation_path.trim().starts_with('$') {
            validate_runtime_expr(v, &op_path, operation_path.trim());
        } else {
            validate_template_string(v, &op_path, operation_path);
        }
        if !operation_path.contains("$sourceDescriptions.") {
            v.push(
                op_path,
                "must use a $sourceDescriptions.* runtime expression to identify the source description document",
            );
        }
    }

    if let Some(outputs) = &step.outputs {
        validate_map_keys(v, &format!("{path}.outputs"), outputs.keys());
        for (k, expr) in outputs {
            validate_runtime_expr(v, &format!("{path}.outputs.{k}"), expr);
        }
    }

    let context = if step.workflow_id.is_some() {
        Some(parameters::ParameterContext::WorkflowStep)
    } else if step.operation_id.is_some() || step.operation_path.is_some() {
        Some(parameters::ParameterContext::OperationStep)
    } else {
        None
    };

    if let Some(parameters) = &step.parameters {
        parameters::validate_parameter_list(v, &format!("{path}.parameters"), parameters, context);
    }

    if let Some(rb) = &step.request_body {
        let rb_path = format!("{path}.requestBody");
        v.validate_extensions(&rb_path, &rb.extensions);
        if let Some(payload) = &rb.payload {
            crate::validate::rules::common::validate_value_exprs(
                v,
                &format!("{rb_path}.payload"),
                payload,
            );
        }
        if let Some(replacements) = &rb.replacements {
            for (ridx, rep) in replacements.iter().enumerate() {
                let rpath = format!("{rb_path}.replacements[{ridx}]");
                v.validate_extensions(&rpath, &rep.extensions);
                if rep.target.trim().is_empty() {
                    v.push(format!("{rpath}.target"), "must not be empty");
                }
                crate::validate::rules::common::validate_value_exprs(
                    v,
                    &format!("{rpath}.value"),
                    &rep.value,
                );
            }
        }
    }

    if let Some(success_criteria) = &step.success_criteria {
        criteria::validate_criteria_list(v, &format!("{path}.successCriteria"), success_criteria);
    }
    if let Some(on_success) = &step.on_success {
        actions::validate_success_action_list(
            v,
            &format!("{path}.onSuccess"),
            on_success,
            Some(step_ids),
        );
    }
    if let Some(on_failure) = &step.on_failure {
        actions::validate_failure_action_list(
            v,
            &format!("{path}.onFailure"),
            on_failure,
            Some(step_ids),
        );
    }
}

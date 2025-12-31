use std::collections::HashSet;

use crate::types::Workflow;
use crate::validate::rules::{
    actions,
    common::{validate_map_keys, validate_runtime_expr},
    parameters, step,
};
use crate::validate::validator::{Validator, ID_RE};

pub(crate) fn validate_workflow(v: &mut Validator, wf: &Workflow, path: &str) {
    if wf.steps.is_empty() {
        v.push(format!("{path}.steps"), "must have at least one entry");
    }

    if let Some(outputs) = &wf.outputs {
        validate_map_keys(v, &format!("{path}.outputs"), outputs.keys());
        for (k, expr) in outputs {
            validate_runtime_expr(v, &format!("{path}.outputs.{k}"), expr);
        }
    }

    if let Some(parameters) = &wf.parameters {
        parameters::validate_parameter_list(v, &format!("{path}.parameters"), parameters, None);
    }

    if let Some(success_actions) = &wf.success_actions {
        actions::validate_success_action_list(
            v,
            &format!("{path}.successActions"),
            success_actions,
            None,
        );
    }
    if let Some(failure_actions) = &wf.failure_actions {
        actions::validate_failure_action_list(
            v,
            &format!("{path}.failureActions"),
            failure_actions,
            None,
        );
    }

    let mut step_ids = HashSet::<String>::new();
    for (idx, s) in wf.steps.iter().enumerate() {
        let spath = format!("{path}.steps[{idx}]");
        v.validate_extensions(&spath, &s.extensions);

        if !ID_RE.is_match(&s.step_id) {
            v.push(
                format!("{spath}.stepId"),
                "must match regex [A-Za-z0-9_\\-]+",
            );
        }
        if !step_ids.insert(s.step_id.clone()) {
            v.push(
                format!("{spath}.stepId"),
                "must be unique within the workflow",
            );
        }

        step::validate_step(v, s, &spath, &step_ids);
    }
}

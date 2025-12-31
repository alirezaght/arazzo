use std::collections::HashSet;

use crate::types::ParameterOrReusable;
use crate::validate::rules::common::{validate_runtime_expr, validate_value_exprs};
use crate::validate::validator::Validator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParameterContext {
    WorkflowStep,
    OperationStep,
}

pub(crate) fn validate_parameter_list(
    v: &mut Validator,
    path: &str,
    params: &[ParameterOrReusable],
    context: Option<ParameterContext>,
) {
    let mut seen = HashSet::<String>::new();
    for (idx, item) in params.iter().enumerate() {
        let ipath = format!("{path}[{idx}]");
        match item {
            ParameterOrReusable::Parameter(p) => {
                v.validate_extensions(&ipath, &p.extensions);
                if p.name.trim().is_empty() {
                    v.push(format!("{ipath}.name"), "must not be empty");
                }
                validate_value_exprs(v, &format!("{ipath}.value"), &p.value);
                match context {
                    Some(ParameterContext::WorkflowStep) => {
                        if p.r#in.is_some() {
                            v.push(
                                format!("{ipath}.in"),
                                "must be omitted when the step specifies workflowId (parameters map to workflow inputs)",
                            );
                        }
                    }
                    Some(ParameterContext::OperationStep) => {
                        if p.r#in.is_none() {
                            v.push(
                                format!("{ipath}.in"),
                                "must be provided when the step targets an operationId/operationPath",
                            );
                        }
                    }
                    None => {}
                }
                let key = format!("param:{}:{:?}", p.name, p.r#in);
                if !seen.insert(key) {
                    v.push(ipath, "duplicate parameter (unique by name + in)");
                }
            }
            ParameterOrReusable::Reusable(r) => {
                let key = format!("ref:{}", r.reference);
                if !seen.insert(key) {
                    v.push(ipath.as_str(), "duplicate reusable reference");
                }
                validate_runtime_expr(v, &format!("{ipath}.reference"), &r.reference);
                if !r.reference.starts_with("$components.parameters.") {
                    v.push(
                        format!("{ipath}.reference"),
                        "must reference $components.parameters.*",
                    );
                }
                if let Some(value) = &r.value {
                    validate_value_exprs(v, &format!("{ipath}.value"), value);
                }
            }
        }
    }
}

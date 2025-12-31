use std::collections::HashSet;

use crate::types::ArazzoDocument;
use crate::validate::rules::{common::validate_runtime_expr, components, info, workflow};
use crate::validate::validator::{ID_RE, Validator};

pub(crate) fn validate_document(v: &mut Validator, doc: &ArazzoDocument) {
    v.validate_extensions("$", &doc.extensions);
    v.validate_spec_version("$.arazzo", &doc.arazzo);

    info::validate_info(v, &doc.info, "$.info");

    if doc.source_descriptions.is_empty() {
        v.push("$.sourceDescriptions", "must have at least one entry");
    }

    let mut source_names = HashSet::<String>::new();
    for (idx, src) in doc.source_descriptions.iter().enumerate() {
        let path = format!("$.sourceDescriptions[{idx}]");
        v.validate_extensions(&path, &src.extensions);

        if !ID_RE.is_match(&src.name) {
            v.push(format!("{path}.name"), "must match regex [A-Za-z0-9_\\-]+");
        }
        if !source_names.insert(src.name.clone()) {
            v.push(format!("{path}.name"), "must be unique");
        }
        if src.url.trim().is_empty() {
            v.push(format!("{path}.url"), "must not be empty");
        }
    }

    if doc.workflows.is_empty() {
        v.push("$.workflows", "must have at least one entry");
    }

    let mut workflow_ids = HashSet::<String>::new();
    for (idx, wf) in doc.workflows.iter().enumerate() {
        let path = format!("$.workflows[{idx}]");
        v.validate_extensions(&path, &wf.extensions);

        if !ID_RE.is_match(&wf.workflow_id) {
            v.push(
                format!("{path}.workflowId"),
                "must match regex [A-Za-z0-9_\\-]+",
            );
        }
        if !workflow_ids.insert(wf.workflow_id.clone()) {
            v.push(format!("{path}.workflowId"), "must be unique");
        }

        workflow::validate_workflow(v, wf, &path);
    }

    // dependsOn: validate against local workflowId unless it's an external runtime expression.
    for (idx, wf) in doc.workflows.iter().enumerate() {
        let path = format!("$.workflows[{idx}]");
        if let Some(depends_on) = &wf.depends_on {
            for (didx, dep) in depends_on.iter().enumerate() {
                let dep_path = format!("{path}.dependsOn[{didx}]");
                if dep.starts_with("$sourceDescriptions.") {
                    validate_runtime_expr(v, &dep_path, dep);
                    continue;
                }
                if !workflow_ids.contains(dep) {
                    v.push(
                        dep_path,
                        "must reference an existing local workflowId (or use a $sourceDescriptions.* runtime expression)",
                    );
                }
            }
        }
    }

    if let Some(c) = &doc.components {
        components::validate_components(v, c, "$.components");
    }
}


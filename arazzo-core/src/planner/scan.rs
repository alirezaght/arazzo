use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::expressions::{parse_runtime_expr, parse_template, Segment};
use crate::types::{AnyValue, Step, Workflow};

static STEPS_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\$steps\\.([A-Za-z0-9_\\-]+)").expect("valid"));
static INPUTS_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\$inputs\\.([a-zA-Z0-9\\.\\-_]+)").expect("valid"));

#[derive(Debug, Default)]
pub(crate) struct ScanResult {
    pub step_dependencies: BTreeMap<String, BTreeSet<String>>,
    pub referenced_inputs_by_step: BTreeMap<String, BTreeSet<String>>,
    pub missing_inputs_by_step: BTreeMap<String, BTreeSet<String>>,
    pub missing_inputs_all: BTreeSet<String>,
}

pub(crate) fn scan_workflow(workflow: &Workflow, inputs: Option<&serde_json::Value>) -> ScanResult {
    let mut out = ScanResult::default();
    for step in &workflow.steps {
        let mut deps = BTreeSet::<String>::new();
        let mut inputs_ref = BTreeSet::<String>::new();

        scan_step(step, &mut deps, &mut inputs_ref);

        out.step_dependencies.insert(step.step_id.clone(), deps);
        out.referenced_inputs_by_step
            .insert(step.step_id.clone(), inputs_ref.clone());

        let missing = compute_missing_inputs(&inputs_ref, inputs);
        if !missing.is_empty() {
            out.missing_inputs_all.extend(missing.iter().cloned());
            out.missing_inputs_by_step
                .insert(step.step_id.clone(), missing);
        }
    }
    out
}

fn scan_step(step: &Step, deps: &mut BTreeSet<String>, inputs_ref: &mut BTreeSet<String>) {
    // parameters
    if let Some(params) = &step.parameters {
        for p in params {
            match p {
                crate::types::ParameterOrReusable::Parameter(p) => {
                    scan_value(&p.value, deps, inputs_ref)
                }
                crate::types::ParameterOrReusable::Reusable(r) => {
                    scan_string(&r.reference, deps, inputs_ref);
                    if let Some(v) = &r.value {
                        scan_value(v, deps, inputs_ref);
                    }
                }
            }
        }
    }

    // outputs values are runtime expressions
    if let Some(outputs) = &step.outputs {
        for v in outputs.values() {
            scan_string(v, deps, inputs_ref);
        }
    }

    // operationId / workflowId / operationPath
    if let Some(op_id) = &step.operation_id {
        scan_string(op_id, deps, inputs_ref);
    }
    if let Some(wf_id) = &step.workflow_id {
        scan_string(wf_id, deps, inputs_ref);
    }
    if let Some(op_path) = &step.operation_path {
        scan_string(op_path, deps, inputs_ref);
    }

    // request body
    if let Some(rb) = &step.request_body {
        if let Some(payload) = &rb.payload {
            scan_value(payload, deps, inputs_ref);
        }
        if let Some(reps) = &rb.replacements {
            for r in reps {
                scan_string(&r.target, deps, inputs_ref);
                scan_value(&r.value, deps, inputs_ref);
            }
        }
    }

    // criteria context + condition (best-effort regex scan)
    if let Some(criteria) = &step.success_criteria {
        for c in criteria {
            if let Some(ctx) = &c.context {
                scan_string(ctx, deps, inputs_ref);
            }
            scan_string(&c.condition, deps, inputs_ref);
        }
    }
}

fn scan_value(value: &AnyValue, deps: &mut BTreeSet<String>, inputs_ref: &mut BTreeSet<String>) {
    match value {
        AnyValue::Null | AnyValue::Bool(_) | AnyValue::Number(_) => {}
        AnyValue::String(s) => scan_string(s, deps, inputs_ref),
        AnyValue::Array(arr) => {
            for v in arr {
                scan_value(v, deps, inputs_ref);
            }
        }
        AnyValue::Object(map) => {
            for (_k, v) in map {
                scan_value(v, deps, inputs_ref);
            }
        }
    }
}

fn scan_string(s: &str, deps: &mut BTreeSet<String>, inputs_ref: &mut BTreeSet<String>) {
    // Full runtime expression
    if let Ok(expr) = parse_runtime_expr(s.trim()) {
        match expr {
            crate::expressions::RuntimeExpr::Steps(np) => {
                deps.insert(np.root);
            }
            crate::expressions::RuntimeExpr::Inputs(np) => {
                inputs_ref.insert(np.root);
            }
            _ => {}
        }
        return;
    }

    // Embedded templates
    if let Ok(tpl) = parse_template(s) {
        for seg in tpl.segments {
            if let Segment::Expr(e) = seg {
                if let Ok(expr) = parse_runtime_expr(&e) {
                    match expr {
                        crate::expressions::RuntimeExpr::Steps(np) => {
                            deps.insert(np.root);
                        }
                        crate::expressions::RuntimeExpr::Inputs(np) => {
                            inputs_ref.insert(np.root);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Best-effort raw regex scan (for Criterion.condition etc.).
    for cap in STEPS_REF_RE.captures_iter(s) {
        if let Some(m) = cap.get(1) {
            deps.insert(m.as_str().to_string());
        }
    }
    for cap in INPUTS_REF_RE.captures_iter(s) {
        if let Some(m) = cap.get(1) {
            inputs_ref.insert(m.as_str().to_string());
        }
    }
}

fn compute_missing_inputs(
    referenced: &BTreeSet<String>,
    inputs: Option<&serde_json::Value>,
) -> BTreeSet<String> {
    let Some(inputs) = inputs else {
        return referenced.clone();
    };

    referenced
        .iter()
        .filter(|name| !input_present(inputs, name))
        .cloned()
        .collect()
}

fn input_present(inputs: &serde_json::Value, name: &str) -> bool {
    // First attempt: direct key in top-level object.
    if let Some(obj) = inputs.as_object() {
        if obj.contains_key(name) {
            return true;
        }
    }

    // Second attempt: treat dots as path separators.
    let mut cur = inputs;
    for seg in name.split('.') {
        let Some(obj) = cur.as_object() else {
            return false;
        };
        let Some(next) = obj.get(seg) else {
            return false;
        };
        cur = next;
    }
    true
}

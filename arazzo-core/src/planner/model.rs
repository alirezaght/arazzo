use std::collections::{BTreeMap, BTreeSet};

use crate::error::ValidationError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanningOutcome {
    pub validation: ValidationSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationSummary {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationSummary {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid_from(err: ValidationError) -> Self {
        let errors = err
            .violations
            .into_iter()
            .map(|v| format!("{}: {}", v.path, v.message))
            .collect();
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    pub summary: PlanSummary,
    pub graph: DependencyGraph,
    pub steps: Vec<PlanIntentStep>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanSummary {
    pub workflow_id: String,
    pub workflow_depends_on: Vec<String>,
    pub missing_inputs: BTreeSet<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyGraph {
    /// For each step, which steps it depends on.
    pub depends_on: BTreeMap<String, Vec<String>>,
    /// Steps grouped by parallelizable “levels”.
    pub levels: Vec<Vec<String>>,
    /// A deterministic topological order.
    pub topo_order: Vec<String>,
}

impl DependencyGraph {
    pub fn to_dot(&self, workflow_id: &str) -> String {
        let mut out = String::new();
        out.push_str("digraph arazzo {\n");
        out.push_str(&format!("  label=\"workflow: {workflow_id}\";\n"));
        out.push_str("  labelloc=t;\n");
        out.push_str("  rankdir=LR;\n");

        for (step, deps) in &self.depends_on {
            if deps.is_empty() {
                out.push_str(&format!("  \"{step}\";\n"));
            } else {
                for dep in deps {
                    out.push_str(&format!("  \"{dep}\" -> \"{step}\";\n"));
                }
            }
        }

        for level in &self.levels {
            if level.len() > 1 {
                out.push_str("  { rank=same; ");
                for s in level {
                    out.push_str(&format!("\"{s}\"; "));
                }
                out.push_str("}\n");
            }
        }

        out.push_str("}\n");
        out
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanIntentStep {
    pub step_id: String,
    pub depends_on: Vec<String>,
    pub operation: PlanOperationRef,
    pub success_criteria: Vec<String>,
    pub failure_actions: Vec<String>,
    pub declared_output_keys: Vec<String>,
    pub referenced_inputs: BTreeSet<String>,
    pub missing_inputs: BTreeSet<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PlanOperationRef {
    OperationId {
        /// The raw `operationId` value (may be a runtime expression referencing a source).
        operation_id: String,
        /// Best-effort extracted source description name (if the operationId is qualified).
        source: Option<String>,
    },
    OperationPath {
        operation_path: String,
        /// Best-effort extracted source description name (if templated).
        source: Option<String>,
    },
    WorkflowCall {
        workflow_id: String,
    },
    Unknown,
}

impl PlanOperationRef {
    pub fn from_step(
        _doc: &crate::types::ArazzoDocument,
        _workflow: &crate::types::Workflow,
        step: &crate::types::Step,
    ) -> Self {
        if let Some(op_id) = &step.operation_id {
            let source = extract_source_from_qualified_expr(op_id);
            return Self::OperationId {
                operation_id: op_id.clone(),
                source,
            };
        }
        if let Some(op_path) = &step.operation_path {
            let source = extract_source_from_templated_op_path(op_path);
            return Self::OperationPath {
                operation_path: op_path.clone(),
                source,
            };
        }
        if let Some(wf_id) = &step.workflow_id {
            return Self::WorkflowCall {
                workflow_id: wf_id.clone(),
            };
        }
        Self::Unknown
    }
}

fn extract_source_from_qualified_expr(s: &str) -> Option<String> {
    // Best-effort: operationId may be `$sourceDescriptions.<name>.<operationId>`
    let trimmed = s.trim();
    if !trimmed.starts_with('$') {
        return None;
    }
    match crate::expressions::parse_runtime_expr(trimmed) {
        Ok(crate::expressions::RuntimeExpr::SourceDescriptions(np)) => Some(np.root),
        _ => None,
    }
}

fn extract_source_from_templated_op_path(operation_path: &str) -> Option<String> {
    // Best-effort: operationPath often contains `{$sourceDescriptions.<name>.url}#...`
    if let Ok(tpl) = crate::expressions::parse_template(operation_path) {
        for seg in tpl.segments {
            if let crate::expressions::Segment::Expr(e) = seg {
                if let Ok(crate::expressions::RuntimeExpr::SourceDescriptions(np)) =
                    crate::expressions::parse_runtime_expr(&e)
                {
                    return Some(np.root);
                }
            }
        }
    }
    None
}

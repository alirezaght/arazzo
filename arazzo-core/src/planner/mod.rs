mod dependency;
mod format;
mod model;
mod scan;

use crate::error::ParseError;
use crate::parser::{parse_document_str, DocumentFormat};
use crate::types::{ArazzoDocument, Workflow};
use crate::validate::validate_document;

pub use format::PlanFormat;
pub use model::{
    DependencyGraph, Plan, PlanIntentStep, PlanOperationRef, PlanSummary, PlanningOutcome,
    ValidationSummary,
};

#[derive(Debug, Clone, Default)]
pub struct PlanOptions {
    /// Which workflow to plan. Required when the document has multiple workflows.
    pub workflow_id: Option<String>,
    /// Optional inputs JSON (used to report missing inputs and pre-validate templates).
    pub inputs: Option<serde_json::Value>,
}

pub fn plan_from_str(
    input: &str,
    doc_format: DocumentFormat,
    options: PlanOptions,
) -> Result<PlanningOutcome, PlannerError> {
    let parsed = parse_document_str(input, doc_format).map_err(PlannerError::Parse)?;
    plan_document(&parsed.document, options)
}

pub fn plan_document(
    doc: &ArazzoDocument,
    options: PlanOptions,
) -> Result<PlanningOutcome, PlannerError> {
    let validation = match validate_document(doc) {
        Ok(()) => ValidationSummary::valid(),
        Err(e) => ValidationSummary::invalid_from(e),
    };

    if !validation.is_valid {
        return Ok(PlanningOutcome {
            validation,
            plan: None,
        });
    }

    let workflow = select_workflow(doc, options.workflow_id.as_deref())?;
    let plan = build_plan(doc, workflow, options.inputs)?;
    Ok(PlanningOutcome {
        validation,
        plan: Some(plan),
    })
}

fn select_workflow<'a>(
    doc: &'a ArazzoDocument,
    workflow_id: Option<&str>,
) -> Result<&'a Workflow, PlannerError> {
    if doc.workflows.len() == 1 {
        return Ok(&doc.workflows[0]);
    }

    let Some(id) = workflow_id else {
        return Err(PlannerError::WorkflowSelectionRequired);
    };

    doc.workflows
        .iter()
        .find(|w| w.workflow_id == id)
        .ok_or_else(|| PlannerError::UnknownWorkflowId(id.to_string()))
}

fn build_plan(
    doc: &ArazzoDocument,
    workflow: &Workflow,
    inputs: Option<serde_json::Value>,
) -> Result<Plan, PlannerError> {
    let scan = scan::scan_workflow(workflow, inputs.as_ref());
    let graph = dependency::build_step_dependency_graph(workflow, &scan.step_dependencies)
        .map_err(PlannerError::DependencyGraph)?;

    let steps = workflow
        .steps
        .iter()
        .map(|s| {
            let deps = graph
                .depends_on
                .get(&s.step_id)
                .cloned()
                .unwrap_or_default();

            let op_ref = PlanOperationRef::from_step(doc, workflow, s);

            PlanIntentStep {
                step_id: s.step_id.clone(),
                depends_on: deps,
                operation: op_ref,
                success_criteria: s
                    .success_criteria
                    .as_ref()
                    .map(|c| c.iter().map(|cc| cc.condition.clone()).collect())
                    .unwrap_or_default(),
                failure_actions: s
                    .on_failure
                    .as_ref()
                    .map(|fa| format::format_failure_actions(fa))
                    .unwrap_or_default(),
                declared_output_keys: s
                    .outputs
                    .as_ref()
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default(),
                referenced_inputs: scan
                    .referenced_inputs_by_step
                    .get(&s.step_id)
                    .cloned()
                    .unwrap_or_default(),
                missing_inputs: scan
                    .missing_inputs_by_step
                    .get(&s.step_id)
                    .cloned()
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();

    Ok(Plan {
        summary: PlanSummary {
            workflow_id: workflow.workflow_id.clone(),
            workflow_depends_on: workflow.depends_on.clone().unwrap_or_default(),
            missing_inputs: scan.missing_inputs_all,
        },
        graph,
        steps,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("workflow selection is required when the document contains multiple workflows")]
    WorkflowSelectionRequired,

    #[error("unknown workflowId: {0}")]
    UnknownWorkflowId(String),

    #[error("unable to build dependency graph: {0}")]
    DependencyGraph(String),
}

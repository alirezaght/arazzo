use std::collections::BTreeSet;

use arazzo_core::types::{ArazzoDocument, ParameterLocation, Step, Workflow};

use crate::openapi::{
    DiagnosticSeverity, OpenApiDiagnostic, OpenApiParamLocation, OpenApiResolver,
    ResolvedOperation, ResolvedSources,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledPlan {
    pub diagnostics: Vec<OpenApiDiagnostic>,
    pub steps: Vec<CompiledStep>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledStep {
    pub step_id: String,
    pub operation: Option<ResolvedOperation>,
    pub diagnostics: Vec<OpenApiDiagnostic>,
    pub missing_required_parameters: Vec<MissingParameter>,
    pub request_body: Option<CompiledRequestBody>,
    pub missing_required_request_body: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MissingParameter {
    pub name: String,
    pub location: OpenApiParamLocation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledRequestBody {
    pub content_type: Option<String>,
    pub required: Option<bool>,
    pub available_content_types: Option<Vec<String>>,
}

#[derive(Default)]
pub struct Compiler {
    resolver: OpenApiResolver,
}

impl Compiler {
    pub async fn compile_workflow(
        &self,
        doc: &ArazzoDocument,
        workflow: &Workflow,
    ) -> CompiledPlan {
        let sources = self.resolver.resolve_sources(doc).await;
        compile_workflow_with_sources(&self.resolver, &sources, workflow).await
    }
}

async fn compile_workflow_with_sources(
    resolver: &OpenApiResolver,
    sources: &ResolvedSources,
    workflow: &Workflow,
) -> CompiledPlan {
    let mut plan = CompiledPlan {
        diagnostics: sources.diagnostics.clone(),
        steps: Vec::new(),
    };

    for step in &workflow.steps {
        let mut diag = Vec::new();
        let mut missing = Vec::new();
        let mut op: Option<ResolvedOperation> = None;
        let mut rb: Option<CompiledRequestBody> = None;
        let mut missing_rb_required = false;

        if step.operation_id.is_some() || step.operation_path.is_some() {
            match resolver
                .resolve_step_operation(sources, workflow, step)
                .await
            {
                Ok((resolved, mut extra_diags)) => {
                    diag.append(&mut extra_diags);
                    missing = missing_required_params(step, &resolved);
                    rb = compiled_request_body(step, &resolved);
                    missing_rb_required = is_required_request_body_missing(step, &resolved);
                    op = Some(resolved);
                }
                Err(e) => {
                    diag.push(e);
                }
            }
        }

        // Promote missing requirements to diagnostics for CI friendliness.
        if !missing.is_empty() {
            diag.push(OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "missing required parameters: {}",
                    missing
                        .iter()
                        .map(|m| format!("{}:{:?}", m.name, m.location))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                source_name: None,
            });
        }
        if missing_rb_required {
            diag.push(OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: "missing required requestBody".to_string(),
                source_name: None,
            });
        }

        plan.steps.push(CompiledStep {
            step_id: step.step_id.clone(),
            operation: op,
            diagnostics: diag,
            missing_required_parameters: missing,
            request_body: rb,
            missing_required_request_body: missing_rb_required,
        });
    }

    plan
}

fn missing_required_params(step: &Step, op: &ResolvedOperation) -> Vec<MissingParameter> {
    let mut provided = BTreeSet::<(OpenApiParamLocation, String)>::new();
    if let Some(params) = &step.parameters {
        for p in params {
            if let arazzo_core::types::ParameterOrReusable::Parameter(p) = p {
                if let Some(loc) = &p.r#in {
                    if let Some(open_loc) = map_param_loc(loc) {
                        provided.insert((open_loc, p.name.clone()));
                    }
                }
            }
        }
    }

    op.shape
        .parameters
        .iter()
        .filter(|p| p.required)
        .filter(|p| !provided.contains(&(p.location, p.name.clone())))
        .map(|p| MissingParameter {
            name: p.name.clone(),
            location: p.location,
        })
        .collect()
}

fn compiled_request_body(step: &Step, op: &ResolvedOperation) -> Option<CompiledRequestBody> {
    if op.shape.request_body_required.is_none() && op.shape.request_body_content_types.is_none() {
        return None;
    }

    let content_type = step
        .request_body
        .as_ref()
        .and_then(|rb| rb.content_type.clone());

    Some(CompiledRequestBody {
        content_type,
        required: op.shape.request_body_required,
        available_content_types: op.shape.request_body_content_types.clone(),
    })
}

fn is_required_request_body_missing(step: &Step, op: &ResolvedOperation) -> bool {
    match op.shape.request_body_required {
        Some(true) => step.request_body.is_none(),
        _ => false,
    }
}

fn map_param_loc(loc: &ParameterLocation) -> Option<OpenApiParamLocation> {
    match loc {
        ParameterLocation::Path => Some(OpenApiParamLocation::Path),
        ParameterLocation::Query => Some(OpenApiParamLocation::Query),
        ParameterLocation::Header => Some(OpenApiParamLocation::Header),
        ParameterLocation::Cookie => Some(OpenApiParamLocation::Cookie),
    }
}

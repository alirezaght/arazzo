use std::collections::BTreeMap;

use arazzo_core::types::{ArazzoDocument, SourceDescriptionType, Step, Workflow};

use crate::openapi::loader::load_openapi;
use crate::openapi::model::{DiagnosticSeverity, OpenApiDiagnostic, OpenApiDoc, ResolvedOperation};
use crate::openapi::op_id::{OperationIdSelection, find_operation_by_id, select_source_for_operation_id};
use crate::openapi::op_path::parse_operation_path_ref;
use crate::openapi::shape::{compile_operation_shape, select_base_url};

#[derive(Debug, Default)]
pub struct ResolvedSources {
    pub openapi_docs: BTreeMap<String, OpenApiDoc>,
    pub diagnostics: Vec<OpenApiDiagnostic>,
}

pub struct OpenApiResolver {
    client: reqwest::Client,
}

impl Default for OpenApiResolver {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl OpenApiResolver {
    pub async fn resolve_sources(&self, doc: &ArazzoDocument) -> ResolvedSources {
        let mut out = ResolvedSources::default();

        for src in &doc.source_descriptions {
            let ty = src.source_type.clone().unwrap_or(SourceDescriptionType::Openapi);
            if ty != SourceDescriptionType::Openapi {
                continue;
            }

            match load_openapi(&self.client, &src.url).await {
                Ok(raw) => {
                    out.openapi_docs.insert(
                        src.name.clone(),
                        OpenApiDoc {
                            source_url: src.url.clone(),
                            raw,
                        },
                    );
                }
                Err(e) => out.diagnostics.push(OpenApiDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("failed to load OpenAPI for source '{}': {e}", src.name),
                    source_name: Some(src.name.clone()),
                }),
            }
        }

        out
    }

    pub async fn resolve_step_operation(
        &self,
        sources: &ResolvedSources,
        workflow: &Workflow,
        step: &Step,
    ) -> Result<(ResolvedOperation, Vec<OpenApiDiagnostic>), OpenApiDiagnostic> {
        let mut diags = Vec::<OpenApiDiagnostic>::new();
        // operationId resolution
        if let Some(op_id) = &step.operation_id {
            let (source_name, operation_id) = match select_source_for_operation_id(sources, workflow, op_id) {
                OperationIdSelection::Selected { source_name, operation_id, warnings } => {
                    for w in warnings {
                        diags.push(OpenApiDiagnostic {
                            severity: DiagnosticSeverity::Warning,
                            message: w,
                            source_name: Some(source_name.clone()),
                        });
                    }
                    (source_name, operation_id)
                }
                OperationIdSelection::Error(m) => {
                    return Err(OpenApiDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: m,
                        source_name: None,
                    })
                }
            };

            let doc = sources.openapi_docs.get(&source_name).ok_or_else(|| OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!("OpenAPI source '{source_name}' is not available"),
                source_name: Some(source_name.clone()),
            })?;

            let (resolved, shape_diags) =
                find_operation_by_id(&doc.raw, &source_name, &operation_id).ok_or_else(|| OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!("operationId '{operation_id}' not found in source '{source_name}'"),
                source_name: Some(source_name.clone()),
            })?;

            for m in shape_diags {
                diags.push(OpenApiDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: m,
                    source_name: Some(source_name.clone()),
                });
            }

            return Ok((resolved, diags));
        }

        // operationPath resolution
        if let Some(op_path) = &step.operation_path {
            let (source_name, pointer, method, path) = parse_operation_path_ref(op_path).map_err(|m| OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: m,
                source_name: None,
            })?;

            let doc = sources.openapi_docs.get(&source_name).ok_or_else(|| OpenApiDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!("OpenAPI source '{source_name}' is not available"),
                source_name: Some(source_name.clone()),
            })?;

            let op_obj = doc
                .raw
                .pointer(&pointer)
                .ok_or_else(|| OpenApiDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("operationPath pointer '{pointer}' not found in source '{source_name}'"),
                    source_name: Some(source_name.clone()),
                })?;

            let base_url = select_base_url(&doc.raw, &path, &method, op_obj).unwrap_or_default();
            let (shape, shape_diags) = compile_operation_shape(&doc.raw, &source_name, &path, &method, op_obj);
            for m in shape_diags {
                diags.push(OpenApiDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: m,
                    source_name: Some(source_name.clone()),
                });
            }

            return Ok((ResolvedOperation {
                source_name,
                base_url,
                method: method.to_uppercase(),
                path,
                operation_id: op_obj.get("operationId").and_then(|v| v.as_str()).map(|s| s.to_string()),
                shape,
            }, diags));
        }

        Err(OpenApiDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: "step does not reference an operation (missing operationId/operationPath)".to_string(),
            source_name: None,
        })
    }
}


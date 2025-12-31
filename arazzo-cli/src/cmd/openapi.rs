use std::path::Path;

use arazzo_core::{parse_document_str, DocumentFormat};
use serde::Serialize;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::{OpenApiArgs, OutputArgs};

#[derive(Serialize)]
struct ResolvedEndpoint {
    step_id: String,
    source: String,
    method: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
}

#[derive(Serialize)]
struct OpenApiResult {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    endpoints: Vec<ResolvedEndpoint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

pub async fn openapi_cmd(path: &Path, output: OutputArgs, _openapi: OpenApiArgs) -> i32 {
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) => {
            print_error(
                output.format,
                output.quiet,
                &format!("failed to read {}: {e}", path.display()),
            );
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let parsed = match parse_document_str(&content, DocumentFormat::Auto) {
        Ok(p) => p,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("{e}"));
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let mut endpoints = Vec::new();
    let mut errors = Vec::new();

    for wf in &parsed.document.workflows {
        let compiled = arazzo_exec::Compiler::default()
            .compile_workflow(&parsed.document, wf)
            .await;

        for d in &compiled.diagnostics {
            if d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error {
                errors.push(d.message.clone());
            }
        }

        for s in &compiled.steps {
            for d in &s.diagnostics {
                if d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error {
                    errors.push(format!("{}: {}", s.step_id, d.message));
                }
            }
            if let Some(op) = &s.operation {
                endpoints.push(ResolvedEndpoint {
                    step_id: s.step_id.clone(),
                    source: op.source_name.clone(),
                    method: op.method.clone(),
                    path: op.path.clone(),
                    operation_id: op.operation_id.clone(),
                });
            }
        }
    }

    let result = OpenApiResult {
        endpoints,
        errors: errors.clone(),
    };

    if output.format == OutputFormat::Text && !output.quiet {
        if !result.errors.is_empty() {
            println!("Errors:");
            for e in &result.errors {
                println!("  - {e}");
            }
            println!();
        }
        println!("Resolved endpoints:");
        for ep in &result.endpoints {
            println!("  {} {} {} ({})", ep.step_id, ep.method, ep.path, ep.source);
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    if errors.is_empty() {
        exit_codes::SUCCESS
    } else {
        exit_codes::VALIDATION_FAILED
    }
}

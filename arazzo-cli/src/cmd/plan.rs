use std::path::Path;

use arazzo_core::{DocumentFormat, PlanOptions, PlanningOutcome, PlanOperationRef, parse_document_str, plan_document};
use serde::Serialize;

use crate::exit_codes;
use crate::output::{OutputFormat, print_error};
use crate::{OutputArgs, OpenApiArgs};

pub async fn plan_cmd(
    path: &Path,
    workflow_id: Option<&str>,
    inputs_path: Option<&Path>,
    compile: bool,
    output: OutputArgs,
    _openapi: OpenApiArgs,
) -> i32 {
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to read {}: {e}", path.display()));
            return exit_codes::RUNTIME_ERROR;
        }
    };

    let inputs = super::config::load_inputs(inputs_path, &output);
    if inputs.is_none() && inputs_path.is_some() {
        return exit_codes::RUNTIME_ERROR;
    }

    let parsed = match parse_document_str(&content, DocumentFormat::Auto) {
        Ok(p) => p,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("{e}"));
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let outcome = match plan_document(
        &parsed.document,
        PlanOptions {
            workflow_id: workflow_id.map(String::from),
            inputs: inputs.clone(),
        },
    ) {
        Ok(o) => o,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("{e}"));
            return exit_codes::VALIDATION_FAILED;
        }
    };

    let compiled = if compile && outcome.validation.is_valid {
        match &outcome.plan {
            None => None,
            Some(plan) => {
                let wf = match parsed
                    .document
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == plan.summary.workflow_id)
                {
                    Some(w) => w,
                    None => {
                        print_error(output.format, output.quiet, &format!("workflow '{}' not found in document", plan.summary.workflow_id));
                        return exit_codes::VALIDATION_FAILED;
                    }
                };

                Some(arazzo_exec::Compiler::default().compile_workflow(&parsed.document, wf).await)
            }
        }
    } else {
        None
    };

    match output.format {
        OutputFormat::Json => print_json(&outcome, compiled.as_ref(), output.quiet),
        OutputFormat::Text => print_text(&outcome, compiled.as_ref(), output.quiet),
        OutputFormat::Dot => print_dot(&outcome, output.quiet),
    }
}

#[derive(Serialize)]
struct PlanJsonOutput<'a> {
    logical: &'a PlanningOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiled: Option<&'a arazzo_exec::CompiledPlan>,
}

fn print_json(outcome: &PlanningOutcome, compiled: Option<&arazzo_exec::CompiledPlan>, quiet: bool) -> i32 {
    if quiet {
        return if outcome.validation.is_valid && !compiled_has_errors(compiled) {
            exit_codes::SUCCESS
        } else {
            exit_codes::VALIDATION_FAILED
        };
    }
    let payload = PlanJsonOutput { logical: outcome, compiled };
    match serde_json::to_string_pretty(&payload) {
        Ok(s) => {
            println!("{s}");
            if !outcome.validation.is_valid || compiled_has_errors(compiled) {
                return exit_codes::VALIDATION_FAILED;
            }
            exit_codes::SUCCESS
        }
        Err(e) => {
            eprintln!("error: failed to serialize plan as JSON: {e}");
            exit_codes::RUNTIME_ERROR
        }
    }
}

fn print_text(outcome: &PlanningOutcome, compiled: Option<&arazzo_exec::CompiledPlan>, quiet: bool) -> i32 {
    if quiet {
        return if outcome.validation.is_valid && !compiled_has_errors(compiled) {
            exit_codes::SUCCESS
        } else {
            exit_codes::VALIDATION_FAILED
        };
    }

    if outcome.validation.is_valid {
        println!("validation: valid");
    } else {
        println!("validation: invalid");
        println!("errors: {}", outcome.validation.errors.len());
        for e in &outcome.validation.errors {
            println!("- {e}");
        }
        return exit_codes::VALIDATION_FAILED;
    }

    let Some(plan) = &outcome.plan else { return exit_codes::VALIDATION_FAILED; };

    if !plan.summary.workflow_depends_on.is_empty() {
        println!("workflow dependsOn: {}", plan.summary.workflow_depends_on.join(", "));
    }
    if !plan.summary.missing_inputs.is_empty() {
        println!("missing inputs: {}", plan.summary.missing_inputs.iter().cloned().collect::<Vec<_>>().join(", "));
    }

    println!("\nexecution levels:");
    for (idx, level) in plan.graph.levels.iter().enumerate() {
        if !level.is_empty() {
            println!("  Level {idx}: {}", level.join(", "));
        }
    }

    println!("\nper-step intent:");
    for s in &plan.steps {
        println!("- stepId: {}", s.step_id);
        if !s.depends_on.is_empty() {
            println!("  dependsOn: {}", s.depends_on.join(", "));
        }
        match &s.operation {
            PlanOperationRef::OperationId { operation_id, source } => {
                if let Some(source) = source {
                    println!("  source: {source}");
                }
                println!("  operationId: {operation_id}");
            }
            PlanOperationRef::OperationPath { operation_path, source } => {
                if let Some(source) = source {
                    println!("  source: {source}");
                }
                println!("  operationPath: {operation_path}");
            }
            PlanOperationRef::WorkflowCall { workflow_id } => {
                println!("  workflowId: {workflow_id}");
            }
            PlanOperationRef::Unknown => {
                println!("  operation: <unknown>");
            }
        }
        if !s.declared_output_keys.is_empty() {
            println!("  outputs: {}", s.declared_output_keys.to_vec().join(", "));
        }
    }

    if let Some(compiled) = compiled {
        println!("\ncompiled (openapi-aware):");
        for d in &compiled.diagnostics {
            println!("- {:?}: {}", d.severity, d.message);
        }
        for s in &compiled.steps {
            println!("- stepId: {}", s.step_id);
            if let Some(op) = &s.operation {
                println!("  http: {} {}", op.method, op.path);
            }
        }
        if compiled_has_errors(Some(compiled)) {
            return exit_codes::VALIDATION_FAILED;
        }
    }

    exit_codes::SUCCESS
}

fn print_dot(outcome: &PlanningOutcome, quiet: bool) -> i32 {
    if quiet {
        return if outcome.validation.is_valid {
            exit_codes::SUCCESS
        } else {
            exit_codes::VALIDATION_FAILED
        };
    }

    if !outcome.validation.is_valid {
        eprintln!("error: cannot generate DOT graph for invalid workflow");
        return exit_codes::VALIDATION_FAILED;
    }

    let Some(plan) = &outcome.plan else {
        eprintln!("error: no plan available");
        return exit_codes::VALIDATION_FAILED;
    };

    println!("{}", plan.graph.to_dot(&plan.summary.workflow_id));
    exit_codes::SUCCESS
}

fn compiled_has_errors(compiled: Option<&arazzo_exec::CompiledPlan>) -> bool {
    let Some(c) = compiled else { return false; };
    if c.diagnostics.iter().any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error) {
        return true;
    }
    c.steps.iter().any(|s| s.diagnostics.iter().any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error))
}

use std::path::Path;

use arazzo_core::{parse_document_str, DocumentFormat};
use serde::Serialize;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::OutputArgs;

#[derive(Serialize)]
struct WorkflowInfo {
    workflow_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    step_count: usize,
}

#[derive(Serialize)]
struct WorkflowsResult {
    workflows: Vec<WorkflowInfo>,
}

pub async fn workflows_cmd(path: &Path, output: OutputArgs) -> i32 {
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

    let workflows: Vec<WorkflowInfo> = parsed
        .document
        .workflows
        .iter()
        .map(|w| WorkflowInfo {
            workflow_id: w.workflow_id.clone(),
            summary: w.summary.clone(),
            description: w.description.clone(),
            step_count: w.steps.len(),
        })
        .collect();

    let result = WorkflowsResult { workflows };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Workflows in {}:", path.display());
        for w in &result.workflows {
            println!("  - {} ({} steps)", w.workflow_id, w.step_count);
            if let Some(s) = &w.summary {
                println!("    {s}");
            }
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}

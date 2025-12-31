use std::path::Path;

use arazzo_core::{parse_document_str, DocumentFormat};
use serde::Serialize;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::OutputArgs;

#[derive(Serialize)]
struct InputInfo {
    name: String,
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
struct StepInfo {
    step_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    output_keys: Vec<String>,
}

#[derive(Serialize)]
struct SourceInfo {
    name: String,
    url: String,
    r#type: String,
}

#[derive(Serialize)]
struct InspectResult {
    workflow_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<InputInfo>,
    steps: Vec<StepInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    output_keys: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sources: Vec<SourceInfo>,
}

pub async fn inspect_cmd(path: &Path, workflow_id: Option<&str>, output: OutputArgs) -> i32 {
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

    let wf = if let Some(id) = workflow_id {
        parsed
            .document
            .workflows
            .iter()
            .find(|w| w.workflow_id == id)
    } else if parsed.document.workflows.len() == 1 {
        parsed.document.workflows.first()
    } else {
        print_error(
            output.format,
            output.quiet,
            "multiple workflows found, use --workflow to select one",
        );
        return exit_codes::VALIDATION_FAILED;
    };

    let Some(wf) = wf else {
        print_error(
            output.format,
            output.quiet,
            &format!("workflow not found: {}", workflow_id.unwrap_or("?")),
        );
        return exit_codes::VALIDATION_FAILED;
    };

    let inputs: Vec<InputInfo> = wf
        .inputs
        .as_ref()
        .map(|schema| {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                props
                    .iter()
                    .map(|(name, prop)| InputInfo {
                        name: name.clone(),
                        r#type: prop
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("any")
                            .to_string(),
                        description: prop
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(String::from),
                    })
                    .collect()
            } else {
                vec![]
            }
        })
        .unwrap_or_default();

    let steps: Vec<StepInfo> = wf
        .steps
        .iter()
        .map(|s| StepInfo {
            step_id: s.step_id.clone(),
            operation_id: s.operation_id.clone(),
            operation_path: s.operation_path.clone(),
            workflow_id: s.workflow_id.clone(),
            depends_on: vec![], // Computed by planner, not available on raw step
            output_keys: s
                .outputs
                .as_ref()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default(),
        })
        .collect();

    let output_keys: Vec<String> = wf
        .outputs
        .as_ref()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();

    let sources: Vec<SourceInfo> = parsed
        .document
        .source_descriptions
        .iter()
        .map(|s| SourceInfo {
            name: s.name.clone(),
            url: s.url.clone(),
            r#type: s
                .source_type
                .clone()
                .map(|t| format!("{t:?}"))
                .unwrap_or_else(|| "openapi".to_string()),
        })
        .collect();

    let result = InspectResult {
        workflow_id: wf.workflow_id.clone(),
        inputs,
        steps,
        output_keys,
        sources,
    };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Workflow: {}", result.workflow_id);
        if !result.inputs.is_empty() {
            println!("\nInputs:");
            for i in &result.inputs {
                println!("  - {} ({})", i.name, i.r#type);
            }
        }
        println!("\nSteps:");
        for s in &result.steps {
            let op = s
                .operation_id
                .as_deref()
                .or(s.operation_path.as_deref())
                .or(s.workflow_id.as_deref())
                .unwrap_or("?");
            println!("  - {} -> {}", s.step_id, op);
        }
        if !result.output_keys.is_empty() {
            println!("\nOutputs: {}", result.output_keys.join(", "));
        }
        if !result.sources.is_empty() {
            println!("\nSources:");
            for s in &result.sources {
                println!("  - {}: {}", s.name, s.url);
            }
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    exit_codes::SUCCESS
}

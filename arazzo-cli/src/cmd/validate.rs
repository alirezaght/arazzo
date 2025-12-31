use std::path::Path;

use arazzo_core::{parse_document_str, DocumentFormat, ParseError, Validate};
use serde::Serialize;

use crate::exit_codes;
use crate::output::{print_error, print_result, OutputFormat};
use crate::OutputArgs;

#[derive(Serialize)]
struct ValidateResult {
    valid: bool,
    format: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

pub async fn validate_cmd(path: &Path, output: OutputArgs) -> i32 {
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
        Err(ParseError::Json(e)) => {
            print_error(
                output.format,
                output.quiet,
                &format!("JSON parse failed: {e}"),
            );
            return exit_codes::VALIDATION_FAILED;
        }
        Err(ParseError::Yaml(e)) => {
            print_error(
                output.format,
                output.quiet,
                &format!("YAML parse failed: {e}"),
            );
            return exit_codes::VALIDATION_FAILED;
        }
        Err(ParseError::UnknownFormat) => {
            print_error(
                output.format,
                output.quiet,
                "input is neither valid JSON nor valid YAML",
            );
            return exit_codes::VALIDATION_FAILED;
        }
    };

    match parsed.document.validate() {
        Ok(()) => {
            let result = ValidateResult {
                valid: true,
                format: format!("{:?}", parsed.format),
                errors: vec![],
            };
            if output.format == OutputFormat::Text && !output.quiet {
                println!("ok: valid Arazzo document ({:?})", parsed.format);
            } else {
                print_result(output.format, output.quiet, &result);
            }
            exit_codes::SUCCESS
        }
        Err(err) => {
            let errors: Vec<String> = err
                .violations
                .iter()
                .map(|v| format!("{}: {}", v.path, v.message))
                .collect();
            let result = ValidateResult {
                valid: false,
                format: format!("{:?}", parsed.format),
                errors: errors.clone(),
            };
            if output.format == OutputFormat::Text && !output.quiet {
                eprintln!("error: validation failed");
                for e in &errors {
                    eprintln!("- {e}");
                }
            } else {
                print_result(output.format, output.quiet, &result);
            }
            exit_codes::VALIDATION_FAILED
        }
    }
}

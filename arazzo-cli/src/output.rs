use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Dot,
}

pub fn print_result<T: Serialize>(format: OutputFormat, quiet: bool, result: &T) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Text => {
            if let Ok(json) = serde_json::to_string_pretty(result) {
                println!("{json}");
            }
        }
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string(result) {
                println!("{json}");
            }
        }
        OutputFormat::Dot => {
            // DOT format is handled by specific commands (e.g., plan)
            // This is a fallback for other commands
            if let Ok(json) = serde_json::to_string_pretty(result) {
                println!("{json}");
            }
        }
    }
}

pub fn print_error(format: OutputFormat, quiet: bool, message: &str) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Text => eprintln!("error: {message}"),
        OutputFormat::Json => {
            let err = serde_json::json!({"error": message});
            eprintln!("{}", serde_json::to_string(&err).unwrap_or_default());
        }
        OutputFormat::Dot => eprintln!("error: {message}"),
    }
}


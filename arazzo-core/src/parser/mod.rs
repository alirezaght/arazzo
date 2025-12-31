use crate::error::ParseError;
use crate::types::ArazzoDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Json,
    Yaml,
    Auto,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub document: ArazzoDocument,
    pub format: DocumentFormat,
}

pub fn parse_document_str(input: &str, format: DocumentFormat) -> Result<ParsedDocument, ParseError> {
    match format {
        DocumentFormat::Json => Ok(ParsedDocument {
            document: serde_json::from_str::<ArazzoDocument>(input)?,
            format,
        }),
        DocumentFormat::Yaml => Ok(ParsedDocument {
            document: serde_yaml::from_str::<ArazzoDocument>(input)?,
            format,
        }),
        DocumentFormat::Auto => parse_document_auto(input),
    }
}

fn parse_document_auto(input: &str) -> Result<ParsedDocument, ParseError> {
    // Heuristic: JSON always starts with `{` or `[` after trimming.
    let trimmed = input.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        match serde_json::from_str::<ArazzoDocument>(input) {
            Ok(doc) => {
                return Ok(ParsedDocument {
                    document: doc,
                    format: DocumentFormat::Json,
                });
            }
            Err(e) => {
                // If JSON parsing fails, try YAML as fallback
                match serde_yaml::from_str::<ArazzoDocument>(input) {
                    Ok(doc) => {
                        return Ok(ParsedDocument {
                            document: doc,
                            format: DocumentFormat::Yaml,
                        });
                    }
                    Err(_) => {
                        // Return JSON error since we tried JSON first
                        return Err(ParseError::Json(e));
                    }
                }
            }
        }
    }

    // Try YAML first for non-JSON-looking input
    match serde_yaml::from_str::<ArazzoDocument>(input) {
        Ok(doc) => {
            Ok(ParsedDocument {
                document: doc,
                format: DocumentFormat::Yaml,
            })
        }
        Err(e) => {
            // If YAML fails, try JSON as fallback
            if let Ok(doc) = serde_json::from_str::<ArazzoDocument>(input) {
                return Ok(ParsedDocument {
                    document: doc,
                    format: DocumentFormat::Json,
                });
            }
            // Return YAML error since we tried YAML first
            Err(ParseError::Yaml(e))
        }
    }
}


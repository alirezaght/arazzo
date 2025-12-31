mod json_pointer;
mod runtime;
mod template;

pub use json_pointer::{JsonPointer, JsonPointerError};
pub use runtime::{parse_runtime_expr, RuntimeExpr, RuntimeExprError, Source};
pub use template::{parse_template, Segment, Template, TemplateError};

use crate::types::AnyValue;

/// Validate that any expression-like strings inside a value are syntactically valid.
///
/// - If a string starts with `$`, it must be a valid runtime expression.
/// - If a string contains embedded `{ $... }` expressions, each embedded expression must be valid.
pub fn validate_value_expressions(value: &AnyValue) -> Result<(), TemplateError> {
    template::validate_value_expressions(value)
}

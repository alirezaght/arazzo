#![forbid(unsafe_code)]

pub mod error;
pub mod expressions;
pub mod parser;
pub mod planner;
pub mod types;
pub mod validate;

pub use crate::error::{ArazzoError, ParseError, ValidationError};
pub use crate::parser::{parse_document_str, DocumentFormat, ParsedDocument};
pub use crate::planner::{
    plan_document, plan_from_str, DependencyGraph, Plan, PlanFormat, PlanIntentStep,
    PlanOperationRef, PlanOptions, PlanSummary, PlanningOutcome, ValidationSummary,
};
pub use crate::types::ArazzoDocument;
pub use crate::validate::{validate_document, Validate};

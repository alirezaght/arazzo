#![forbid(unsafe_code)]

pub mod error;
pub mod expressions;
pub mod parser;
pub mod planner;
pub mod types;
pub mod validate;

pub use crate::error::{ArazzoError, ParseError, ValidationError};
pub use crate::parser::{DocumentFormat, ParsedDocument, parse_document_str};
pub use crate::types::ArazzoDocument;
pub use crate::validate::{Validate, validate_document};
pub use crate::planner::{
    DependencyGraph, Plan, PlanFormat, PlanIntentStep, PlanOperationRef, PlanOptions,
    PlanningOutcome, PlanSummary, ValidationSummary, plan_from_str, plan_document,
};

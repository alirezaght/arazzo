#![forbid(unsafe_code)]

//! Runtime engine for executing Arazzo workflows.
//!
//! This crate is intentionally thin for now; the spec parsing/validation lives in `arazzo-core`.

pub mod openapi;
pub mod compile;
pub mod secrets;
pub mod policy;
pub mod retry;
pub mod executor;

pub use crate::compile::{CompiledPlan, CompiledRequestBody, CompiledStep, Compiler, MissingParameter};
pub use crate::executor::Executor;

pub struct Engine;

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self
    }
}

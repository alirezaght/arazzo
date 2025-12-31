#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub succeeded_steps: usize,
    pub failed_steps: usize,
    pub retries_scheduled: usize,
}

impl ExecutionResult {
    pub fn record_success(&mut self) {
        self.succeeded_steps += 1;
    }

    pub fn record_retry(&mut self) {
        self.retries_scheduled += 1;
    }

    pub fn record_failure(&mut self) {
        self.failed_steps += 1;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("store error: {0}")]
    Store(#[from] arazzo_store::StoreError),
    #[error("step not found: {0}")]
    StepNotFound(String),
    #[error("compiled step not found: {0}")]
    CompiledStepNotFound(String),
    #[error("missing operation for step: {0}")]
    MissingOperation(String),
    #[error("task join error: {0}")]
    TaskJoin(String),
}

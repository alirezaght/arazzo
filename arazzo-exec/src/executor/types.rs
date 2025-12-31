use std::collections::BTreeMap;
use std::time::Duration;

use crate::policy::PolicyConfig;
use crate::retry::RetryConfig;

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub global_concurrency: usize,
    pub per_source_concurrency: BTreeMap<String, usize>,
    pub poll_interval: Duration,
    pub policy: PolicyConfig,
    pub retry: RetryConfig,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            global_concurrency: 10,
            per_source_concurrency: BTreeMap::new(),
            poll_interval: Duration::from_millis(200),
            policy: PolicyConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionOutcome {
    pub succeeded_steps: usize,
    pub failed_steps: usize,
    pub retries_scheduled: usize,
}


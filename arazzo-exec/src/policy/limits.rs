use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct LimitsConfig {
    pub request: RequestLimits,
    pub response: ResponseLimits,
    pub run: RunLimitsConfig,
}

#[derive(Debug, Clone)]
pub struct RequestLimits {
    pub max_body_bytes: usize,
    pub max_headers_count: usize,
    pub max_headers_bytes: usize,
}

impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 1024 * 1024, // 1MB
            max_headers_count: 100,
            max_headers_bytes: 16 * 1024, // 16KB
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseLimits {
    pub max_body_bytes: usize,
    pub max_headers_count: usize,
    pub max_headers_bytes: usize,
}

impl Default for ResponseLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 4 * 1024 * 1024, // 4MB
            max_headers_count: 100,
            max_headers_bytes: 32 * 1024, // 32KB
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunLimitsConfig {
    pub max_steps_per_run: usize,
    pub max_concurrent_steps: usize,
    pub max_total_run_time: Option<Duration>,
}

impl Default for RunLimitsConfig {
    fn default() -> Self {
        Self {
            max_steps_per_run: 1_000,
            max_concurrent_steps: 10,
            max_total_run_time: None,
        }
    }
}

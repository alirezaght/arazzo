use std::path::PathBuf;

use clap::Args;

use crate::output::OutputFormat;

#[derive(Debug, Args, Clone)]
pub struct OutputArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text, global = true)]
    pub format: OutputFormat,
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Args, Clone)]
pub struct StoreArgs {
    #[arg(long)]
    pub store: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct OpenApiArgs {
    #[arg(long = "openapi", value_name = "NAME=PATH")]
    pub openapi_sources: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct SecretsArgs {
    #[arg(long, default_value = "env")]
    pub secrets: String,
}

#[derive(Debug, Args, Clone)]
pub struct WebhookArgs {
    #[arg(long)]
    pub webhook_url: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct PolicyArgs {
    #[arg(long = "allow-host")]
    pub allow_hosts: Vec<String>,
    #[arg(long)]
    pub allow_hosts_file: Option<PathBuf>,
    #[arg(long)]
    pub allow_http: bool,
    #[arg(long)]
    pub follow_redirects: bool,
    #[arg(long, default_value_t = 5)]
    pub max_redirects: usize,
    #[arg(long, default_value_t = 30000)]
    pub timeout: u64,
    #[arg(long, default_value_t = 4_194_304)]
    pub max_response_bytes: usize,
    #[arg(long, default_value_t = 4_194_304)]
    pub max_request_bytes: usize,
    #[arg(long, default_value_t = 100)]
    pub max_headers_count: usize,
    #[arg(long, default_value_t = 1000)]
    pub max_steps_per_run: usize,
    #[arg(long, default_value_t = 100)]
    pub max_concurrent_steps: usize,
    #[arg(long, default_value_t = 3600)]
    pub max_run_time_seconds: u64,
}

#[derive(Debug, Args, Clone)]
pub struct ConcurrencyArgs {
    #[arg(long, default_value_t = 10)]
    pub max_concurrency: usize,
    #[arg(long = "max-concurrency-source", value_name = "NAME=N")]
    pub max_concurrency_source: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct RetryArgs {
    #[arg(long)]
    pub retry_max_attempts: Option<usize>,
    #[arg(long)]
    pub retry_max_delay: Option<u64>,
    #[arg(long, default_value = "full")]
    pub retry_jitter: String,
}

use std::path::PathBuf;

use clap::Subcommand;

use crate::args::*;

#[derive(Debug, Subcommand)]
pub enum Command {
    Execute {
        path: PathBuf,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        inputs: Option<PathBuf>,
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set_inputs: Vec<String>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[arg(long, default_value = "postgres")]
        events: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
        #[command(flatten)]
        openapi: OpenApiArgs,
        #[command(flatten)]
        secrets: SecretsArgs,
        #[command(flatten)]
        webhook: WebhookArgs,
        #[command(flatten)]
        policy: PolicyArgs,
        #[command(flatten)]
        concurrency: ConcurrencyArgs,
        #[command(flatten)]
        retry: RetryArgs,
    },
    Start {
        path: PathBuf,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        inputs: Option<PathBuf>,
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set_inputs: Vec<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
        #[command(flatten)]
        openapi: OpenApiArgs,
        #[command(flatten)]
        secrets: SecretsArgs,
        #[command(flatten)]
        policy: PolicyArgs,
        #[command(flatten)]
        concurrency: ConcurrencyArgs,
        #[command(flatten)]
        retry: RetryArgs,
    },
    Resume {
        run_id: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
        #[command(flatten)]
        secrets: SecretsArgs,
        #[command(flatten)]
        policy: PolicyArgs,
        #[command(flatten)]
        concurrency: ConcurrencyArgs,
        #[command(flatten)]
        retry: RetryArgs,
    },
    Cancel {
        run_id: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
    },
    Status {
        run_id: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
    },
    Trace {
        run_id: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
    },
    Events {
        run_id: String,
        #[arg(long, short)]
        follow: bool,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
    },
    Validate {
        path: PathBuf,
        #[command(flatten)]
        output: OutputArgs,
    },
    Plan {
        path: PathBuf,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        inputs: Option<PathBuf>,
        #[arg(long, alias = "resolve-openapi")]
        compile: bool,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        openapi: OpenApiArgs,
    },
    Workflows {
        path: PathBuf,
        #[command(flatten)]
        output: OutputArgs,
    },
    Inspect {
        path: PathBuf,
        #[arg(long)]
        workflow: Option<String>,
        #[command(flatten)]
        output: OutputArgs,
    },
    Openapi {
        path: PathBuf,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        openapi: OpenApiArgs,
    },
    Migrate {
        #[command(flatten)]
        store: StoreArgs,
        #[arg(long, default_value_t = 5)]
        max_connections: u32,
        #[command(flatten)]
        output: OutputArgs,
    },
    Doctor {
        #[command(flatten)]
        store: StoreArgs,
        #[command(flatten)]
        openapi: OpenApiArgs,
        #[command(flatten)]
        secrets: SecretsArgs,
        #[command(flatten)]
        policy: PolicyArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    Metrics {
        run_id: String,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        store: StoreArgs,
    },
}

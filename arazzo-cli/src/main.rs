use clap::Parser;

mod args;
mod cmd;
mod commands;
mod exit_codes;
mod output;
mod utils;

pub use args::*;
use commands::Command;

#[derive(Debug, Parser)]
#[command(name = "arazzo", version, about = "Arazzo workflow executor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() {
    let cli = Cli::parse();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to create tokio runtime: {e}");
            std::process::exit(exit_codes::RUNTIME_ERROR);
        }
    };

    let exit_code = rt.block_on(run_command(cli.command));
    std::process::exit(exit_code);
}

async fn run_command(command: Command) -> i32 {
    match command {
        Command::Execute {
            path,
            workflow,
            inputs,
            set_inputs,
            run_id,
            idempotency_key,
            events,
            output,
            store,
            openapi,
            secrets,
            webhook,
            policy,
            concurrency,
            retry,
        } => {
            cmd::execute::execute_cmd(
                &path,
                workflow.as_deref(),
                inputs.as_deref(),
                &set_inputs,
                run_id.as_deref(),
                idempotency_key.as_deref(),
                &events,
                output,
                store,
                openapi,
                secrets,
                webhook,
                policy,
                concurrency,
                retry,
            )
            .await
        }
        Command::Start {
            path,
            workflow,
            inputs,
            set_inputs,
            idempotency_key,
            output,
            store,
            openapi,
            secrets,
            policy,
            concurrency,
            retry,
        } => {
            cmd::start::start_cmd(
                &path,
                workflow.as_deref(),
                inputs.as_deref(),
                &set_inputs,
                idempotency_key.as_deref(),
                output,
                store,
                openapi,
                secrets,
                policy,
                concurrency,
                retry,
            )
            .await
        }
        Command::Resume {
            run_id,
            output,
            store,
            secrets,
            policy,
            concurrency,
            retry,
        } => {
            cmd::resume::resume_cmd(&run_id, output, store, secrets, policy, concurrency, retry)
                .await
        }
        Command::Cancel {
            run_id,
            output,
            store,
        } => cmd::cancel::cancel_cmd(&run_id, output, store).await,
        Command::Status {
            run_id,
            output,
            store,
        } => cmd::status::status_cmd(&run_id, output, store).await,
        Command::Trace {
            run_id,
            output,
            store,
        } => cmd::trace::trace_cmd(&run_id, output, store).await,
        Command::Events {
            run_id,
            follow,
            output,
            store,
        } => cmd::events::events_cmd(&run_id, follow, output, store).await,
        Command::Validate { path, output } => cmd::validate::validate_cmd(&path, output).await,
        Command::Plan {
            path,
            workflow,
            inputs,
            compile,
            output,
            openapi,
        } => {
            cmd::plan::plan_cmd(
                &path,
                workflow.as_deref(),
                inputs.as_deref(),
                compile,
                output,
                openapi,
            )
            .await
        }
        Command::Workflows { path, output } => cmd::workflows::workflows_cmd(&path, output).await,
        Command::Inspect {
            path,
            workflow,
            output,
        } => cmd::inspect::inspect_cmd(&path, workflow.as_deref(), output).await,
        Command::Openapi {
            path,
            output,
            openapi,
        } => cmd::openapi::openapi_cmd(&path, output, openapi).await,
        Command::Migrate {
            store,
            max_connections,
            output,
        } => cmd::migrate::migrate_cmd(store, max_connections, output).await,
        Command::Doctor {
            store,
            openapi,
            secrets,
            policy,
            output,
        } => cmd::doctor::doctor_cmd(store, openapi, secrets, policy, output).await,
        Command::Metrics {
            run_id,
            output,
            store,
        } => cmd::metrics::metrics_cmd(&run_id, output, store).await,
    }
}

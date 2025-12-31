use serde::Serialize;

use crate::exit_codes;
use crate::output::{OutputFormat, print_result};
use crate::{OutputArgs, StoreArgs, OpenApiArgs, SecretsArgs, PolicyArgs};

#[derive(Serialize)]
struct Check {
    name: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct DoctorResult {
    checks: Vec<Check>,
    all_passed: bool,
}

pub async fn doctor_cmd(
    store: StoreArgs,
    _openapi: OpenApiArgs,
    secrets: SecretsArgs,
    policy: PolicyArgs,
    output: OutputArgs,
) -> i32 {
    let mut checks = Vec::new();

    // Check database connection
    let db_check = check_database(&store).await;
    checks.push(db_check);

    // Check secrets provider
    let secrets_check = check_secrets(&secrets);
    checks.push(secrets_check);

    // Check policy configuration
    let policy_check = check_policy(&policy);
    checks.push(policy_check);

    let all_passed = checks.iter().all(|c| c.status == "ok");
    let result = DoctorResult { checks, all_passed };

    if output.format == OutputFormat::Text && !output.quiet {
        println!("Environment checks:");
        for c in &result.checks {
            let icon = if c.status == "ok" { "✓" } else { "✗" };
            print!("  {} {}: {}", icon, c.name, c.status);
            if let Some(msg) = &c.message {
                print!(" - {msg}");
            }
            println!();
        }
        if result.all_passed {
            println!("\nAll checks passed.");
        } else {
            println!("\nSome checks failed.");
        }
    } else {
        print_result(output.format, output.quiet, &result);
    }

    if all_passed {
        exit_codes::SUCCESS
    } else {
        exit_codes::RUNTIME_ERROR
    }
}

async fn check_database(store: &StoreArgs) -> Check {
    let url = store.store.clone()
        .or_else(|| std::env::var("ARAZZO_DATABASE_URL").ok())
        .or_else(|| std::env::var("DATABASE_URL").ok());

    match url {
        None => Check {
            name: "database".to_string(),
            status: "warning".to_string(),
            message: Some("no database URL configured".to_string()),
        },
        Some(url) => {
            match arazzo_store::PostgresStore::connect(&url, 1).await {
                Ok(_) => Check {
                    name: "database".to_string(),
                    status: "ok".to_string(),
                    message: Some("connected".to_string()),
                },
                Err(e) => Check {
                    name: "database".to_string(),
                    status: "error".to_string(),
                    message: Some(format!("connection failed: {e}")),
                },
            }
        }
    }
}

fn check_secrets(secrets: &SecretsArgs) -> Check {
    match secrets.secrets.as_str() {
        "env" => Check {
            name: "secrets".to_string(),
            status: "ok".to_string(),
            message: Some("using environment variables".to_string()),
        },
        s if s.starts_with("file:") => {
            let path = &s[5..];
            if std::path::Path::new(path).exists() {
                Check {
                    name: "secrets".to_string(),
                    status: "ok".to_string(),
                    message: Some(format!("file provider: {path}")),
                }
            } else {
                Check {
                    name: "secrets".to_string(),
                    status: "error".to_string(),
                    message: Some(format!("secrets directory not found: {path}")),
                }
            }
        }
        other => Check {
            name: "secrets".to_string(),
            status: "ok".to_string(),
            message: Some(format!("provider: {other}")),
        },
    }
}

fn check_policy(policy: &PolicyArgs) -> Check {
    if policy.allow_hosts.is_empty() && policy.allow_hosts_file.is_none() {
        Check {
            name: "policy".to_string(),
            status: "warning".to_string(),
            message: Some("no allowed hosts configured (all requests will be denied)".to_string()),
        }
    } else {
        let count = policy.allow_hosts.len();
        Check {
            name: "policy".to_string(),
            status: "ok".to_string(),
            message: Some(format!("{count} allowed hosts configured")),
        }
    }
}


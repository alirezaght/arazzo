use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use crate::{ConcurrencyArgs, OutputArgs, PolicyArgs, RetryArgs};
use crate::output::print_error;

pub fn load_inputs(path: Option<&Path>, output: &OutputArgs) -> Option<serde_json::Value> {
    let path = path?;
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            print_error(output.format, output.quiet, &format!("failed to read inputs: {e}"));
            return None;
        }
    };
    if let Ok(v) = serde_json::from_str(&content) {
        return Some(v);
    }
    if let Ok(v) = serde_yaml::from_str(&content) {
        return Some(v);
    }
    print_error(output.format, output.quiet, "inputs file is neither valid JSON nor YAML");
    None
}

pub fn merge_set_inputs(inputs: &mut Option<serde_json::Value>, set_inputs: &[String]) {
    if set_inputs.is_empty() {
        return;
    }
    let obj = inputs.get_or_insert(serde_json::json!({}));
    if let Some(map) = obj.as_object_mut() {
        for s in set_inputs {
            if let Some((k, v)) = s.split_once('=') {
                map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
            }
        }
    }
}

pub fn build_executor_config(
    concurrency: &ConcurrencyArgs,
    retry: &RetryArgs,
) -> arazzo_exec::executor::ExecutorConfig {
    let mut per_source = BTreeMap::new();
    for s in &concurrency.max_concurrency_source {
        if let Some((name, n)) = s.split_once('=') {
            if let Ok(n) = n.parse() {
                per_source.insert(name.to_string(), n);
            }
        }
    }

    arazzo_exec::executor::ExecutorConfig {
        global_concurrency: concurrency.max_concurrency,
        per_source_concurrency: per_source,
        poll_interval: Duration::from_millis(100),
        policy: arazzo_exec::policy::PolicyConfig::default(),
        retry: arazzo_exec::retry::RetryConfig {
            max_attempts: retry.retry_max_attempts.unwrap_or(5),
            max_delay: Duration::from_millis(retry.retry_max_delay.unwrap_or(60_000)),
            ..Default::default()
        },
    }
}

pub fn build_policy_config(policy: &PolicyArgs) -> arazzo_exec::policy::PolicyConfig {
    let mut hosts: BTreeSet<String> = policy.allow_hosts.iter().cloned().collect();
    if let Some(file) = &policy.allow_hosts_file {
        if let Ok(content) = std::fs::read_to_string(file) {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    hosts.insert(line.to_string());
                }
            }
        }
    }

    let schemes = if policy.allow_http {
        ["https", "http"].into_iter().map(String::from).collect()
    } else {
        ["https"].into_iter().map(String::from).collect()
    };

    arazzo_exec::policy::PolicyConfig {
        network: arazzo_exec::policy::NetworkConfig {
            allowed_schemes: schemes,
            allowed_hosts: hosts,
            allowed_base_urls: BTreeSet::new(),
            redirects: arazzo_exec::policy::RedirectPolicy {
                follow: policy.follow_redirects,
                max_redirects: policy.max_redirects,
            },
            deny_private_ip_literals: true,
        },
        limits: arazzo_exec::policy::LimitsConfig {
            request: arazzo_exec::policy::RequestLimits {
                max_body_bytes: policy.max_request_bytes,
                max_headers_count: policy.max_headers_count,
                max_headers_bytes: 16 * 1024, // Keep reasonable default for header size
            },
            response: arazzo_exec::policy::ResponseLimits {
                max_body_bytes: policy.max_response_bytes,
                max_headers_count: policy.max_headers_count,
                max_headers_bytes: 32 * 1024, // Keep reasonable default for header size
            },
            run: arazzo_exec::policy::RunLimitsConfig {
                max_steps_per_run: policy.max_steps_per_run,
                max_concurrent_steps: policy.max_concurrent_steps,
                max_total_run_time: Some(Duration::from_secs(policy.max_run_time_seconds)),
            },
        },
        ..Default::default()
    }
}

pub fn get_database_url(store_arg: Option<String>, output: &OutputArgs) -> Option<String> {
    let url = store_arg
        .or_else(|| std::env::var("ARAZZO_DATABASE_URL").ok())
        .or_else(|| std::env::var("DATABASE_URL").ok());
    if url.is_none() {
        print_error(output.format, output.quiet, 
            "missing database URL. Set --store <url>, ARAZZO_DATABASE_URL, or DATABASE_URL environment variable");
    }
    url
}


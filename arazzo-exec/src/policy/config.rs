use std::collections::BTreeMap;
use std::time::Duration;

use crate::policy::{LimitsConfig, NetworkConfig, SensitiveHeadersConfig};

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct PolicyConfig {
    pub network: NetworkConfig,
    pub limits: LimitsConfig,
    pub sensitive_headers: SensitiveHeadersConfig,
    /// Default: secrets not allowed in URL path/query.
    pub allow_secrets_in_url: bool,

    /// Per-source overrides keyed by `sourceDescriptions[].name`.
    pub per_source: BTreeMap<String, SourcePolicyConfig>,
}


#[derive(Debug, Clone, Default)]
pub struct SourcePolicyConfig {
    pub network: Option<NetworkConfig>,
    pub limits: Option<LimitsConfig>,
    pub sensitive_headers: Option<SensitiveHeadersConfig>,
    /// Override the global secrets policy for this source.
    pub allow_secrets_in_url: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct PolicyOverrides {
    /// Safe runtime overrides (e.g. tighten limits). We intentionally do not support widening allowlists here.
    pub max_concurrent_steps: Option<usize>,
    pub max_total_run_time: Option<Duration>,
}

impl PolicyConfig {
    pub fn effective_for_source(&self, source: &str, overrides: &PolicyOverrides) -> EffectivePolicy {
        let mut network = self.network.clone();
        let mut limits = self.limits.clone();
        let mut sensitive_headers = self.sensitive_headers.clone();

        if let Some(src) = self.per_source.get(source) {
            if let Some(n) = &src.network {
                network = n.clone();
            }
            if let Some(l) = &src.limits {
                limits = l.clone();
            }
            if let Some(s) = &src.sensitive_headers {
                sensitive_headers = s.clone();
            }
        }

        if let Some(v) = overrides.max_concurrent_steps {
            limits.run.max_concurrent_steps = limits.run.max_concurrent_steps.min(v);
        }
        if let Some(v) = overrides.max_total_run_time {
            limits.run.max_total_run_time = Some(
                limits
                    .run
                    .max_total_run_time
                    .map(|cur| cur.min(v))
                    .unwrap_or(v),
            );
        }

        let allow_secrets_in_url = self
            .per_source
            .get(source)
            .and_then(|s| s.allow_secrets_in_url)
            .unwrap_or(self.allow_secrets_in_url);

        EffectivePolicy {
            network,
            limits,
            sensitive_headers,
            allow_secrets_in_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EffectivePolicy {
    pub network: NetworkConfig,
    pub limits: LimitsConfig,
    pub sensitive_headers: SensitiveHeadersConfig,
    pub allow_secrets_in_url: bool,
}



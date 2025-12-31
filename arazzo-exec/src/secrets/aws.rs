//! AWS Secrets Manager provider.
//!
//! Enabled via the `aws-secrets` feature.
//!
//! # Secret Reference Format
//! - `aws-sm://secret-name` - fetch latest version
//! - `aws-sm://secret-name?version=abc` - fetch specific version
//! - `aws-sm://secret-name?stage=AWSCURRENT` - fetch by staging label

use async_trait::async_trait;
use aws_sdk_secretsmanager::Client;

use crate::secrets::{SecretError, SecretRef, SecretValue, SecretsProvider};

pub struct AwsSecretsManagerProvider {
    client: Client,
    scheme: String,
}

impl AwsSecretsManagerProvider {
    /// Create from an existing SDK client.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            scheme: "aws-sm".to_string(),
        }
    }

    /// Create with default AWS config (env vars, instance metadata, etc.).
    pub async fn from_env() -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        Self::new(client)
    }

    /// Create with custom scheme (e.g., "secrets" to unify with other providers).
    pub fn with_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into();
        self
    }
}

#[async_trait]
impl SecretsProvider for AwsSecretsManagerProvider {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        if secret_ref.scheme != self.scheme {
            return Err(SecretError::NotFound(secret_ref.clone()));
        }

        let mut req = self.client.get_secret_value().secret_id(&secret_ref.id);

        // Parse query params for version/stage
        if let Some(ref query) = secret_ref.query {
            for (k, v) in parse_query(query) {
                match k.as_str() {
                    "version" | "version_id" => {
                        req = req.version_id(v);
                    }
                    "stage" | "version_stage" => {
                        req = req.version_stage(v);
                    }
                    _ => {}
                }
            }
        }

        let resp = req
            .send()
            .await
            .map_err(|e| SecretError::provider(secret_ref.clone(), e.to_string()))?;

        // AWS returns either SecretString or SecretBinary
        if let Some(s) = resp.secret_string() {
            return Ok(SecretValue::from_string(s.to_string()));
        }
        if let Some(b) = resp.secret_binary() {
            return Ok(SecretValue::from_bytes(b.as_ref().to_vec()));
        }

        Err(SecretError::provider(
            secret_ref.clone(),
            "secret has no value".to_string(),
        ))
    }
}

fn parse_query(q: &str) -> Vec<(String, String)> {
    q.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?.to_string();
            let v = parts.next().unwrap_or("").to_string();
            Some((k, v))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_works() {
        let q = "version=abc&stage=AWSCURRENT";
        let pairs = parse_query(q);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("version".to_string(), "abc".to_string()));
        assert_eq!(pairs[1], ("stage".to_string(), "AWSCURRENT".to_string()));
    }
}

//! GCP Secret Manager provider.
//!
//! Enabled via the `gcp-secrets` feature.
//!
//! # Secret Reference Format
//! - `gcp-sm://projects/PROJECT/secrets/SECRET` - fetch latest version
//! - `gcp-sm://projects/PROJECT/secrets/SECRET/versions/VERSION` - fetch specific version

use async_trait::async_trait;
use google_cloud_secretmanager_v1::client::SecretManagerService;

use crate::secrets::{SecretError, SecretRef, SecretValue, SecretsProvider};

pub struct GcpSecretManagerProvider {
    client: SecretManagerService,
    scheme: String,
}

impl GcpSecretManagerProvider {
    /// Create from an existing client.
    pub fn new(client: SecretManagerService) -> Self {
        Self {
            client,
            scheme: "gcp-sm".to_string(),
        }
    }

    /// Create with default GCP config (ADC, metadata server, etc.).
    pub async fn from_env() -> Result<Self, SecretError> {
        let client = SecretManagerService::builder()
            .build()
            .await
            .map_err(|e| SecretError::Provider {
                secret_ref: SecretRef {
                    scheme: "gcp-sm".to_string(),
                    id: "".to_string(),
                    query: None,
                },
                message: format!("failed to create GCP client: {e}"),
            })?;

        Ok(Self::new(client))
    }

    /// Create with custom scheme.
    pub fn with_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into();
        self
    }
}

#[async_trait]
impl SecretsProvider for GcpSecretManagerProvider {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        if secret_ref.scheme != self.scheme {
            return Err(SecretError::NotFound(secret_ref.clone()));
        }

        // The id should be a full resource name:
        // projects/PROJECT/secrets/SECRET/versions/VERSION
        // or projects/PROJECT/secrets/SECRET (we append /versions/latest)
        let name = if secret_ref.id.contains("/versions/") {
            secret_ref.id.clone()
        } else {
            format!("{}/versions/latest", secret_ref.id)
        };

        let resp = self
            .client
            .access_secret_version()
            .set_name(&name)
            .send()
            .await
            .map_err(|e| SecretError::provider(secret_ref.clone(), e.to_string()))?;

        let payload = resp
            .payload
            .ok_or_else(|| SecretError::provider(secret_ref.clone(), "secret has no payload"))?;

        Ok(SecretValue::from_bytes(payload.data.to_vec()))
    }
}

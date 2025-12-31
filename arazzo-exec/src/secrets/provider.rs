use std::collections::BTreeMap;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::secrets::{SecretError, SecretRef, SecretValue};

#[async_trait]
pub trait SecretsProvider: Send + Sync {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError>;

    async fn get_many(
        &self,
        refs: &[SecretRef],
    ) -> Result<BTreeMap<SecretRef, SecretValue>, SecretError> {
        let mut out = BTreeMap::new();
        for r in refs {
            out.insert(r.clone(), self.get(r).await?);
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct CompositeProvider {
    providers: Vec<Box<dyn SecretsProvider>>,
}

impl CompositeProvider {
    pub fn new(providers: Vec<Box<dyn SecretsProvider>>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl SecretsProvider for CompositeProvider {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        for p in &self.providers {
            match p.get(secret_ref).await {
                Ok(v) => return Ok(v),
                Err(SecretError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(SecretError::NotFound(secret_ref.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct EnvSecretsProvider {
    /// scheme to match, usually "secrets"
    pub scheme: String,
    /// Optional prefix to apply to env var lookups.
    pub env_prefix: Option<String>,
}

impl Default for EnvSecretsProvider {
    fn default() -> Self {
        Self {
            scheme: "secrets".to_string(),
            env_prefix: None,
        }
    }
}

#[async_trait]
impl SecretsProvider for EnvSecretsProvider {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        if secret_ref.scheme != self.scheme {
            return Err(SecretError::NotFound(secret_ref.clone()));
        }
        let key = match &self.env_prefix {
            None => secret_ref.id.clone(),
            Some(p) => format!("{p}{}", secret_ref.id),
        };
        match std::env::var(&key) {
            Ok(v) => Ok(SecretValue::from_string(v)),
            Err(std::env::VarError::NotPresent) => Err(SecretError::NotFound(secret_ref.clone())),
            Err(e) => Err(SecretError::provider(secret_ref.clone(), e.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileSecretsProvider {
    /// scheme to match, e.g. "file-secrets"
    pub scheme: String,
    /// base directory; secret id becomes a relative path under this directory.
    pub base_dir: PathBuf,
}

#[async_trait]
impl SecretsProvider for FileSecretsProvider {
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        if secret_ref.scheme != self.scheme {
            return Err(SecretError::NotFound(secret_ref.clone()));
        }
        let path = self.base_dir.join(&secret_ref.id);
        let bytes = std::fs::read(&path).map_err(|e| SecretError::provider(secret_ref.clone(), e.to_string()))?;
        Ok(SecretValue::from_bytes(bytes))
    }
}


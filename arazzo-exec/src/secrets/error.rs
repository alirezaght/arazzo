use crate::secrets::SecretRef;

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("secret not found: {0}")]
    NotFound(SecretRef),
    #[error("secret provider error for {secret_ref}: {message}")]
    Provider {
        secret_ref: SecretRef,
        message: String,
    },
}

impl SecretError {
    pub fn provider(secret_ref: SecretRef, message: impl Into<String>) -> Self {
        Self::Provider {
            secret_ref,
            message: message.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecretPolicyError {
    #[error("secret {secret_ref} is not allowed in {placement:?}")]
    DisallowedPlacement {
        secret_ref: SecretRef,
        placement: super::policy::SecretPlacement,
    },
}


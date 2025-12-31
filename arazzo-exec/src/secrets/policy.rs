use crate::secrets::{SecretPolicyError, SecretRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretPlacement {
    Header,
    Body,
    UrlPath,
    UrlQuery,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SecretsPolicy {
    pub allow_secrets_in_url: bool,
}


impl SecretsPolicy {
    pub fn ensure_allowed(
        &self,
        secret_ref: &SecretRef,
        placement: SecretPlacement,
    ) -> Result<(), SecretPolicyError> {
        match placement {
            SecretPlacement::Header | SecretPlacement::Body => Ok(()),
            SecretPlacement::UrlPath | SecretPlacement::UrlQuery => {
                if self.allow_secrets_in_url {
                    Ok(())
                } else {
                    Err(SecretPolicyError::DisallowedPlacement {
                        secret_ref: secret_ref.clone(),
                        placement,
                    })
                }
            }
        }
    }
}


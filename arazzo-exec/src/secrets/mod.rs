pub mod cache;
mod error;
mod policy;
mod provider;
mod redact;
mod r#ref;
mod value;

#[cfg(feature = "aws-secrets")]
mod aws;
#[cfg(feature = "gcp-secrets")]
mod gcp;

pub use cache::{CacheConfig, CachingProvider};
pub use error::{SecretError, SecretPolicyError};
pub use policy::{SecretPlacement, SecretsPolicy};
pub use provider::{CompositeProvider, EnvSecretsProvider, FileSecretsProvider, SecretsProvider};
pub use redact::{redact_headers, RedactedHeaders, RedactionPolicy};
pub use r#ref::{SecretRef, SecretRefParseError};
pub use value::SecretValue;

#[cfg(feature = "aws-secrets")]
pub use aws::AwsSecretsManagerProvider;
#[cfg(feature = "gcp-secrets")]
pub use gcp::GcpSecretManagerProvider;


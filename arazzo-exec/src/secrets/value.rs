use std::sync::Arc;

use zeroize::Zeroizing;

/// Secret bytes that are not `Debug`/`Display` printable and are zeroized on drop.
#[derive(Clone)]
pub struct SecretValue(Arc<Zeroizing<Vec<u8>>>);

impl SecretValue {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Arc::new(Zeroizing::new(bytes)))
    }

    pub fn from_string(s: String) -> Self {
        // Store UTF-8 bytes.
        Self::from_bytes(s.into_bytes())
    }

    pub fn expose_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValue(<redacted>)")
    }
}

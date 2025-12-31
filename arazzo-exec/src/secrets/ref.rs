use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretRef {
    pub scheme: String,
    pub id: String,
    /// Optional query string (e.g., "version=abc&stage=AWSCURRENT").
    pub query: Option<String>,
}

impl SecretRef {
    pub fn parse(input: &str) -> Result<Self, SecretRefParseError> {
        let s = input.trim();
        let (scheme, rest) = s
            .split_once("://")
            .ok_or(SecretRefParseError::MissingScheme)?;
        if scheme.is_empty() {
            return Err(SecretRefParseError::EmptyScheme);
        }
        if !is_valid_scheme(scheme) {
            return Err(SecretRefParseError::InvalidScheme(scheme.to_string()));
        }

        let (id, query) = match rest.split_once('?') {
            Some((id_part, q)) => (id_part.to_string(), Some(q.to_string())),
            None => (rest.to_string(), None),
        };

        if id.is_empty() {
            return Err(SecretRefParseError::EmptyId);
        }
        Ok(Self {
            scheme: scheme.to_string(),
            id,
            query,
        })
    }

    pub fn as_uri_string(&self) -> String {
        match &self.query {
            Some(q) => format!("{}://{}?{}", self.scheme, self.id, q),
            None => format!("{}://{}", self.scheme, self.id),
        }
    }
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // safe to display: this is an identifier, not a secret value.
        match &self.query {
            Some(q) => write!(f, "{}://{}?{}", self.scheme, self.id, q),
            None => write!(f, "{}://{}", self.scheme, self.id),
        }
    }
}

fn is_valid_scheme(s: &str) -> bool {
    // URI scheme: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SecretRefParseError {
    #[error("secret reference must be URI-like (e.g. secrets://NAME)")]
    MissingScheme,
    #[error("secret reference scheme must not be empty")]
    EmptyScheme,
    #[error("invalid secret reference scheme: {0}")]
    InvalidScheme(String),
    #[error("secret reference id must not be empty")]
    EmptyId,
}

use std::collections::BTreeSet;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub retry_statuses: BTreeSet<u16>,
    pub base_delay: Duration,
    pub factor: f64,
    pub max_delay: Duration,
    pub headers: RetryHeadersConfig,
    pub max_attempts: usize,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            retry_statuses: [429u16, 503, 502, 504, 408].into_iter().collect(),
            base_delay: Duration::from_millis(1000),
            factor: 2.0,
            max_delay: Duration::from_secs(60),
            headers: RetryHeadersConfig::default(),
            max_attempts: 5,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct RetryHeadersConfig {
    /// Vendor-specific retry-after headers (per source, configurable later).
    pub vendor_headers: Vec<RetryVendorHeader>,
}


#[derive(Debug, Clone)]
pub struct RetryVendorHeader {
    pub name: String,
    pub kind: VendorHeaderKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorHeaderKind {
    /// delta seconds
    DeltaSeconds,
    /// unix epoch seconds
    UnixSeconds,
    /// HTTP-date
    HttpDate,
}


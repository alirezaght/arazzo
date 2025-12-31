use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Allowed URL schemes. Defaults to https only.
    pub allowed_schemes: BTreeSet<String>,
    /// Allowed hosts/domains. If empty, requests are denied (secure-by-default).
    pub allowed_hosts: BTreeSet<String>,
    /// Optional per-source base URLs (not enforced yet; reserved for stricter policy).
    pub allowed_base_urls: BTreeSet<String>,
    /// Follow redirects?
    pub redirects: RedirectPolicy,
    /// Deny literal private IPs in host (SSRF guard).
    pub deny_private_ip_literals: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            allowed_schemes: ["https"].into_iter().map(|s| s.to_string()).collect(),
            allowed_hosts: BTreeSet::new(),
            allowed_base_urls: BTreeSet::new(),
            redirects: RedirectPolicy::default(),
            deny_private_ip_literals: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RedirectPolicy {
    pub follow: bool,
    pub max_redirects: usize,
}

pub(crate) fn host_allowed(allowed_hosts: &BTreeSet<String>, host: &str) -> bool {
    if allowed_hosts.is_empty() {
        return false;
    }
    // Exact match or subdomain match (e.g. allow "example.com" matches "api.example.com").
    if allowed_hosts.contains(host) {
        return true;
    }
    allowed_hosts
        .iter()
        .any(|h| host.ends_with(&format!(".{h}")))
}

pub(crate) fn is_private_ip_literal(host: &str) -> bool {
    // Only checks if host is a literal IP (no DNS resolution).
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                let o = v4.octets();
                // 10/8
                if o[0] == 10 {
                    return true;
                }
                // 127/8
                if o[0] == 127 {
                    return true;
                }
                // 192.168/16
                if o[0] == 192 && o[1] == 168 {
                    return true;
                }
                // 172.16/12
                if o[0] == 172 && (16..=31).contains(&o[1]) {
                    return true;
                }
                // link-local 169.254/16
                if o[0] == 169 && o[1] == 254 {
                    return true;
                }
                false
            }
            std::net::IpAddr::V6(v6) => {
                // ::1 loopback, fe80::/10 link-local, fc00::/7 unique local.
                v6.is_loopback()
                    || (v6.segments()[0] & 0xffc0 == 0xfe80) // fe80::/10 link-local
                    || v6.segments()[0] & 0xfe00 == 0xfc00
            }
        }
    } else {
        false
    }
}

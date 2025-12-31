use std::collections::BTreeMap;

use crate::policy::config::{EffectivePolicy, PolicyConfig, PolicyOverrides};
use crate::policy::network::{host_allowed, is_private_ip_literal};
use crate::policy::sanitize::{redact_body_with_secrets, sanitize_headers, truncate_body};

#[derive(Debug, Clone)]
pub struct HttpRequestParts {
    pub method: String,
    pub url: url::Url,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponseParts {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PolicyOutcome {
    pub persistable_request: Option<RequestGateResult>,
    pub persistable_response: Option<ResponseGateResult>,
}

#[derive(Debug, Clone)]
pub struct RequestGateResult {
    pub url: String,
    pub method: String,
    pub headers: super::sanitize::SanitizedHeaders,
    pub body: super::sanitize::SanitizedBody,
}

#[derive(Debug, Clone)]
pub struct ResponseGateResult {
    pub status: u16,
    pub headers: super::sanitize::SanitizedHeaders,
    pub body: super::sanitize::SanitizedBody,
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyGateError {
    #[error("disallowed URL scheme: {0}")]
    Scheme(String),
    #[error("disallowed host: {0}")]
    Host(String),
    #[error("private IP literal disallowed: {0}")]
    PrivateIp(String),
    #[error("request body exceeds max bytes ({len} > {max})")]
    RequestBodyTooLarge { len: usize, max: usize },
    #[error("response body exceeds max bytes ({len} > {max})")]
    ResponseBodyTooLarge { len: usize, max: usize },
    #[error("too many headers ({count} > {max})")]
    HeaderCount { count: usize, max: usize },
    #[error("headers exceed max bytes ({bytes} > {max})")]
    HeaderBytes { bytes: usize, max: usize },
}

pub struct PolicyGate {
    cfg: PolicyConfig,
    overrides: PolicyOverrides,
}

impl PolicyGate {
    pub fn new(cfg: PolicyConfig) -> Self {
        Self {
            cfg,
            overrides: PolicyOverrides::default(),
        }
    }

    pub fn with_overrides(mut self, overrides: PolicyOverrides) -> Self {
        self.overrides = overrides;
        self
    }

    pub fn effective_for_source(&self, source: &str, overrides: &PolicyOverrides) -> EffectivePolicy {
        self.cfg.effective_for_source(source, overrides)
    }

    pub fn apply_request(
        &self,
        source: &str,
        req: &HttpRequestParts,
        secret_derived_header_names: &[String],
        body_contains_secrets: bool,
    ) -> Result<RequestGateResult, PolicyGateError> {
        let eff = self.cfg.effective_for_source(source, &self.overrides);
        enforce_request(&eff, req)?;

        let body = if body_contains_secrets {
            redact_body_with_secrets(&req.body, eff.limits.request.max_body_bytes)
        } else {
            truncate_body(&req.body, eff.limits.request.max_body_bytes)
        };

        Ok(RequestGateResult {
            url: req.url.to_string(),
            method: req.method.clone(),
            headers: sanitize_headers(&req.headers, &eff.sensitive_headers, secret_derived_header_names),
            body,
        })
    }

    pub fn apply_response(
        &self,
        source: &str,
        resp: &HttpResponseParts,
        secret_derived_header_names: &[String],
    ) -> Result<ResponseGateResult, PolicyGateError> {
        let eff = self.cfg.effective_for_source(source, &self.overrides);
        enforce_response(&eff, resp)?;

        Ok(ResponseGateResult {
            status: resp.status,
            headers: sanitize_headers(&resp.headers, &eff.sensitive_headers, secret_derived_header_names),
            body: truncate_body(&resp.body, eff.limits.response.max_body_bytes),
        })
    }
}

fn enforce_request(eff: &EffectivePolicy, req: &HttpRequestParts) -> Result<(), PolicyGateError> {
    let scheme = req.url.scheme().to_string();
    if !eff.network.allowed_schemes.contains(&scheme) {
        return Err(PolicyGateError::Scheme(scheme));
    }

    let host = req.url.host_str().unwrap_or("").to_string();
    if host.is_empty() || !host_allowed(&eff.network.allowed_hosts, &host) {
        return Err(PolicyGateError::Host(host));
    }
    if eff.network.deny_private_ip_literals && is_private_ip_literal(&host) {
        return Err(PolicyGateError::PrivateIp(host));
    }

    enforce_headers(
        &req.headers,
        eff.limits.request.max_headers_count,
        eff.limits.request.max_headers_bytes,
    )?;

    if req.body.len() > eff.limits.request.max_body_bytes {
        return Err(PolicyGateError::RequestBodyTooLarge {
            len: req.body.len(),
            max: eff.limits.request.max_body_bytes,
        });
    }
    Ok(())
}

fn enforce_response(eff: &EffectivePolicy, resp: &HttpResponseParts) -> Result<(), PolicyGateError> {
    enforce_headers(
        &resp.headers,
        eff.limits.response.max_headers_count,
        eff.limits.response.max_headers_bytes,
    )?;

    if resp.body.len() > eff.limits.response.max_body_bytes {
        return Err(PolicyGateError::ResponseBodyTooLarge {
            len: resp.body.len(),
            max: eff.limits.response.max_body_bytes,
        });
    }
    Ok(())
}

fn enforce_headers(
    headers: &BTreeMap<String, String>,
    max_count: usize,
    max_bytes: usize,
) -> Result<(), PolicyGateError> {
    if headers.len() > max_count {
        return Err(PolicyGateError::HeaderCount {
            count: headers.len(),
            max: max_count,
        });
    }
    let bytes: usize = headers
        .iter()
        .map(|(k, v)| k.len() + v.len())
        .sum();
    if bytes > max_bytes {
        return Err(PolicyGateError::HeaderBytes { bytes, max: max_bytes });
    }
    Ok(())
}


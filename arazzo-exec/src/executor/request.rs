use std::collections::BTreeMap;

use arazzo_core::types::{ArazzoDocument, Parameter, ParameterOrReusable, Step};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::executor::eval::{eval_value, EvalContext};
use crate::policy::HttpRequestParts;
use crate::secrets::{SecretPlacement, SecretRef, SecretsProvider};

pub struct RequestBuildResult {
    pub parts: HttpRequestParts,
    pub secret_derived_headers: Vec<String>,
    pub body_contains_secrets: bool,
}

#[derive(Default)]
pub struct SecretsPolicyForSource {
    pub allow_secrets_in_url: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn build_request(
    store: &dyn arazzo_store::StateStore,
    secrets: &dyn SecretsProvider,
    secrets_policy: &SecretsPolicyForSource,
    run_id: Uuid,
    step: &Step,
    resolved_op: &crate::openapi::ResolvedOperation,
    inputs: &JsonValue,
    document: Option<&ArazzoDocument>,
) -> Result<RequestBuildResult, String> {
    let mut headers = BTreeMap::<String, String>::new();
    let mut query = Vec::<(String, String)>::new();
    let mut path_params = BTreeMap::<String, String>::new();
    let mut secret_derived_headers = Vec::<String>::new();

    if let Some(params) = &step.parameters {
        for param_or_ref in params {
            let p = resolve_parameter(param_or_ref, document)?;
            if let Some(p) = p {
                let val = eval_value(
                    &p.value,
                    &EvalContext {
                        run_id,
                        inputs,
                        store,
                        response: None,
                    },
                )
                .await
                .map_err(|e| format!("eval error: {e}"))?;

                let s = value_to_string(&val);
                match &p.r#in {
                    Some(arazzo_core::types::ParameterLocation::Header) => {
                        let (val, is_secret) =
                            resolve_secret(secrets, &s, SecretPlacement::Header, true).await;
                        headers.insert(p.name.clone(), val);
                        if is_secret {
                            secret_derived_headers.push(p.name.clone());
                        }
                    }
                    Some(arazzo_core::types::ParameterLocation::Query) => {
                        let allowed = secrets_policy.allow_secrets_in_url;
                        let (val, _) =
                            resolve_secret(secrets, &s, SecretPlacement::UrlQuery, allowed).await;
                        query.push((p.name.clone(), val));
                    }
                    Some(arazzo_core::types::ParameterLocation::Path) => {
                        let allowed = secrets_policy.allow_secrets_in_url;
                        let (val, _) =
                            resolve_secret(secrets, &s, SecretPlacement::UrlPath, allowed).await;
                        path_params.insert(p.name.clone(), val);
                    }
                    Some(arazzo_core::types::ParameterLocation::Cookie) => {
                        let (val, is_secret) =
                            resolve_secret(secrets, &s, SecretPlacement::Header, true).await;
                        headers
                            .entry("Cookie".to_string())
                            .and_modify(|c| {
                                c.push_str("; ");
                                c.push_str(&format!("{}={}", p.name, val));
                            })
                            .or_insert_with(|| format!("{}={}", p.name, val));
                        if is_secret {
                            secret_derived_headers.push("Cookie".to_string());
                        }
                    }
                    None => {}
                }
            }
        }
    }

    fn resolve_parameter<'a>(
        param_or_ref: &'a ParameterOrReusable,
        document: Option<&'a ArazzoDocument>,
    ) -> Result<Option<&'a Parameter>, String> {
        match param_or_ref {
            ParameterOrReusable::Parameter(p) => Ok(Some(p)),
            ParameterOrReusable::Reusable(r) => {
                // Parse reference like $components.parameters.authHeader
                let ref_str = r.reference.trim();
                if let Some(name) = ref_str.strip_prefix("$components.parameters.") {
                    let doc = document.ok_or_else(|| {
                        "document required to resolve component references".to_string()
                    })?;
                    let components = doc.components.as_ref().ok_or_else(|| {
                        format!("no components defined for reference {}", ref_str)
                    })?;
                    let params = components.parameters.as_ref().ok_or_else(|| {
                        format!("no parameters in components for reference {}", ref_str)
                    })?;
                    let param = params
                        .get(name)
                        .ok_or_else(|| format!("parameter {} not found in components", name))?;
                    Ok(Some(param))
                } else {
                    Err(format!("unsupported parameter reference: {}", ref_str))
                }
            }
        }
    }

    let (body_bytes, body_contains_secrets) = if let Some(rb) = &step.request_body {
        if let Some(payload) = &rb.payload {
            let v = eval_value(
                payload,
                &EvalContext {
                    run_id,
                    inputs,
                    store,
                    response: None,
                },
            )
            .await
            .map_err(|e| format!("eval error: {e}"))?;
            resolve_body_secrets(secrets, v).await?
        } else {
            (Vec::new(), false)
        }
    } else {
        (Vec::new(), false)
    };

    let url = build_url(
        &resolved_op.base_url,
        &resolved_op.path,
        &path_params,
        &query,
    )?;

    Ok(RequestBuildResult {
        parts: HttpRequestParts {
            method: resolved_op.method.clone(),
            url,
            headers,
            body: body_bytes,
        },
        secret_derived_headers,
        body_contains_secrets,
    })
}

async fn resolve_body_secrets(
    secrets: &dyn SecretsProvider,
    value: JsonValue,
) -> Result<(Vec<u8>, bool), String> {
    let (resolved, has_secrets) = resolve_json_secrets(secrets, value).await;
    let bytes = serde_json::to_vec(&resolved)
        .map_err(|e| format!("failed to serialize request body: {e}"))?;
    Ok((bytes, has_secrets))
}

async fn resolve_json_secrets(
    secrets: &dyn SecretsProvider,
    value: JsonValue,
) -> (JsonValue, bool) {
    match value {
        JsonValue::String(s) => {
            if let Ok(r) = SecretRef::parse(&s) {
                if let Ok(v) = secrets.get(&r).await {
                    let resolved = String::from_utf8_lossy(v.expose_bytes()).to_string();
                    return (JsonValue::String(resolved), true);
                }
            }
            (JsonValue::String(s), false)
        }
        JsonValue::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            let mut any_secret = false;
            for v in arr {
                let (resolved, has) = Box::pin(resolve_json_secrets(secrets, v)).await;
                any_secret |= has;
                out.push(resolved);
            }
            (JsonValue::Array(out), any_secret)
        }
        JsonValue::Object(map) => {
            let mut out = serde_json::Map::new();
            let mut any_secret = false;
            for (k, v) in map {
                let (resolved, has) = Box::pin(resolve_json_secrets(secrets, v)).await;
                any_secret |= has;
                out.insert(k, resolved);
            }
            (JsonValue::Object(out), any_secret)
        }
        other => (other, false),
    }
}

async fn resolve_secret(
    secrets: &dyn SecretsProvider,
    s: &str,
    _placement: SecretPlacement,
    allowed: bool,
) -> (String, bool) {
    if !allowed {
        return (s.to_string(), false);
    }
    if let Ok(r) = SecretRef::parse(s) {
        if let Ok(v) = secrets.get(&r).await {
            return (String::from_utf8_lossy(v.expose_bytes()).to_string(), true);
        }
    }
    (s.to_string(), false)
}

fn value_to_string(v: &JsonValue) -> String {
    match v {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => String::new(),
        other => other.to_string(),
    }
}

fn build_url(
    base_url: &str,
    path_template: &str,
    path_params: &BTreeMap<String, String>,
    query: &[(String, String)],
) -> Result<url::Url, String> {
    if base_url.is_empty() {
        return Err("missing OpenAPI server base_url".to_string());
    }
    let mut path = path_template.to_string();
    for (k, v) in path_params {
        path = path.replace(&format!("{{{k}}}"), &urlencoding::encode(v));
    }
    let mut url = url::Url::parse(base_url).map_err(|e| e.to_string())?;
    url.set_path(&path);
    {
        let mut qp = url.query_pairs_mut();
        for (k, v) in query {
            qp.append_pair(k, v);
        }
    }
    Ok(url)
}

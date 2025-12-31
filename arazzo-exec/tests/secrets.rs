use std::collections::BTreeMap;
use std::time::Duration;

use arazzo_exec::secrets::{
    CacheConfig, CachingProvider, RedactionPolicy, SecretPlacement, SecretRef,
    SecretsPolicy, SecretsProvider, redact_headers,
};

#[tokio::test]
async fn secret_ref_parses_uri_like_format() {
    let r = SecretRef::parse("secrets://prod/auth_api_token").unwrap();
    assert_eq!(r.scheme, "secrets");
    assert_eq!(r.id, "prod/auth_api_token");
    assert_eq!(r.to_string(), "secrets://prod/auth_api_token");
}

#[test]
fn policy_disallows_secrets_in_url_by_default() {
    let policy = SecretsPolicy::default();
    let r = SecretRef::parse("secrets://MY_TOKEN").unwrap();
    assert!(policy.ensure_allowed(&r, SecretPlacement::Header).is_ok());
    assert!(policy.ensure_allowed(&r, SecretPlacement::Body).is_ok());
    assert!(policy.ensure_allowed(&r, SecretPlacement::UrlQuery).is_err());
}

#[test]
fn redaction_strips_auth_cookie_and_secret_derived_headers() {
    let mut headers = BTreeMap::new();
    headers.insert("Authorization".to_string(), "Bearer abc".to_string());
    headers.insert("Cookie".to_string(), "a=b".to_string());
    headers.insert("X-Api-Key".to_string(), "k".to_string());

    let out = redact_headers(
        &headers,
        &RedactionPolicy::default(),
        &vec!["X-Api-Key".to_string()],
    );
    assert_eq!(out.headers["Authorization"], "<redacted>");
    assert_eq!(out.headers["Cookie"], "<redacted>");
    assert_eq!(out.headers["X-Api-Key"], "<redacted>");
}

#[tokio::test]
async fn caching_provider_caches_with_ttl() {
    #[derive(Default)]
    struct CountingProvider(std::sync::atomic::AtomicUsize);

    #[async_trait::async_trait]
    impl SecretsProvider for CountingProvider {
        async fn get(&self, _secret_ref: &SecretRef) -> Result<arazzo_exec::secrets::SecretValue, arazzo_exec::secrets::SecretError> {
            let n = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(arazzo_exec::secrets::SecretValue::from_string(format!("v{}", n)))
        }
    }

    let inner = CountingProvider::default();
    let cache = CachingProvider::new(
        inner,
        CacheConfig {
            ttl: Duration::from_millis(50),
            max_entries: 10,
        },
    );
    let r = SecretRef::parse("secrets://anything").unwrap();
    let v_cached = cache.get(&r).await.unwrap();
    assert_eq!(std::str::from_utf8(v_cached.expose_bytes()).unwrap(), "v0");
    tokio::time::sleep(Duration::from_millis(60)).await;
    let v2 = cache.get(&r).await.unwrap();
    assert_eq!(std::str::from_utf8(v2.expose_bytes()).unwrap(), "v1");
}


use std::time::Duration;

use arazzo_exec::secrets::cache::{CacheConfig, CachingProvider};
use arazzo_exec::secrets::{SecretError, SecretRef, SecretValue, SecretsProvider};
use async_trait::async_trait;

struct CountingProvider {
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl SecretsProvider for CountingProvider {
    async fn get(&self, _secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        let n = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(SecretValue::from_string(format!("value-{}", n)))
    }
}

#[tokio::test]
async fn caching_provider_caches_values() {
    let inner = CountingProvider {
        count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let cache = CachingProvider::new(
        inner,
        CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 10,
        },
    );

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "test".to_string(),
        query: None,
    };

    let v1 = cache.get(&secret_ref).await.unwrap();
    let v2 = cache.get(&secret_ref).await.unwrap();
    assert_eq!(v1.expose_bytes(), v2.expose_bytes());
}

#[tokio::test]
async fn caching_provider_expires_after_ttl() {
    let inner = CountingProvider {
        count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let cache = CachingProvider::new(
        inner,
        CacheConfig {
            ttl: Duration::from_millis(50),
            max_entries: 10,
        },
    );

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "test".to_string(),
        query: None,
    };

    let v1 = cache.get(&secret_ref).await.unwrap();
    tokio::time::sleep(Duration::from_millis(60)).await;
    let v2 = cache.get(&secret_ref).await.unwrap();
    assert_ne!(v1.expose_bytes(), v2.expose_bytes());
}

#[tokio::test]
async fn caching_provider_enforces_max_entries() {
    let inner = CountingProvider {
        count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let cache = CachingProvider::new(
        inner,
        CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 2,
        },
    );

    let ref1 = SecretRef {
        scheme: "secrets".to_string(),
        id: "secret1".to_string(),
        query: None,
    };
    let ref2 = SecretRef {
        scheme: "secrets".to_string(),
        id: "secret2".to_string(),
        query: None,
    };
    let ref3 = SecretRef {
        scheme: "secrets".to_string(),
        id: "secret3".to_string(),
        query: None,
    };

    let v1 = cache.get(&ref1).await.unwrap();
    let v2 = cache.get(&ref2).await.unwrap();
    let _ = cache.get(&ref3).await.unwrap();

    let v1_again = cache.get(&ref1).await.unwrap();
    assert_ne!(v1_again.expose_bytes(), v1.expose_bytes(), "ref1 should have been evicted and get a new value");
}

#[tokio::test]
async fn caching_provider_single_flight() {
    let inner = CountingProvider {
        count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let cache = CachingProvider::new(
        inner,
        CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 10,
        },
    );

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "test".to_string(),
        query: None,
    };

    let (v1, v2) = tokio::join!(
        cache.get(&secret_ref),
        cache.get(&secret_ref)
    );

    assert_eq!(v1.unwrap().expose_bytes(), v2.unwrap().expose_bytes());
}


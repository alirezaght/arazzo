use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, Notify};

use crate::secrets::{SecretError, SecretRef, SecretValue, SecretsProvider};

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub ttl: Duration,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            max_entries: 256,
        }
    }
}

pub struct CachingProvider<P> {
    inner: P,
    config: CacheConfig,
    state: Mutex<State>,
}

struct State {
    cache: HashMap<SecretRef, CacheEntry>,
    inflight: HashMap<SecretRef, Arc<Notify>>,
}

struct CacheEntry {
    value: Arc<SecretValue>,
    expires_at: Instant,
    last_accessed: Instant,
}

impl<P> CachingProvider<P>
where
    P: SecretsProvider,
{
    pub fn new(inner: P, config: CacheConfig) -> Self {
        Self {
            inner,
            config,
            state: Mutex::new(State {
                cache: HashMap::new(),
                inflight: HashMap::new(),
            }),
        }
    }
}

#[async_trait::async_trait]
impl<P> SecretsProvider for CachingProvider<P>
where
    P: SecretsProvider,
{
    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, SecretError> {
        // Fast path: cached and not expired.
        {
            let mut s = self.state.lock().await;
            if let Some(entry) = s.cache.get_mut(secret_ref) {
                if Instant::now() < entry.expires_at {
                    entry.last_accessed = Instant::now();
                    return Ok((*entry.value).clone());
                }
            }

            // Single-flight: if another task is already fetching, wait.
            if let Some(n) = s.inflight.get(secret_ref) {
                let n = n.clone();
                drop(s);
                n.notified().await;
                // After notification, try cache again.
                let mut s = self.state.lock().await;
                if let Some(entry) = s.cache.get_mut(secret_ref) {
                    if Instant::now() < entry.expires_at {
                        entry.last_accessed = Instant::now();
                        return Ok((*entry.value).clone());
                    }
                }
                // Fallthrough to fetch if still missing/expired.
            } else {
                s.inflight
                    .insert(secret_ref.clone(), Arc::new(Notify::new()));
            }
        }

        // Fetch outside lock.
        let fetched = self.inner.get(secret_ref).await;

        // Store + notify.
        let notify = {
            let mut s = self.state.lock().await;
            let notify = s
                .inflight
                .remove(secret_ref)
                .unwrap_or_else(|| Arc::new(Notify::new()));

            if let Ok(value) = &fetched {
                enforce_capacity(&mut s.cache, self.config.max_entries);
                let now = Instant::now();
                s.cache.insert(
                    secret_ref.clone(),
                    CacheEntry {
                        value: Arc::new(value.clone()),
                        expires_at: now + self.config.ttl,
                        last_accessed: now,
                    },
                );
            }

            notify
        };

        notify.notify_waiters();
        fetched
    }
}

fn enforce_capacity(cache: &mut HashMap<SecretRef, CacheEntry>, max_entries: usize) {
    if cache.len() < max_entries {
        return;
    }
    // Remove expired entries first
    let now = Instant::now();
    cache.retain(|_, v| v.expires_at > now);

    // If still over capacity, evict least recently used
    while cache.len() >= max_entries {
        let oldest = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, _)| k.clone());
        if let Some(k) = oldest {
            cache.remove(&k);
        } else {
            break;
        }
    }
}

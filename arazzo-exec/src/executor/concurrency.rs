use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub struct ConcurrencyLimits {
    global: Arc<Semaphore>,
    per_source: Arc<HashMap<String, Arc<Semaphore>>>,
}

impl ConcurrencyLimits {
    pub fn new(
        global_limit: usize,
        per_source_limits: &std::collections::BTreeMap<String, usize>,
    ) -> Self {
        Self {
            global: Arc::new(Semaphore::new(global_limit)),
            per_source: Arc::new(
                per_source_limits
                    .iter()
                    .map(|(k, v)| (k.clone(), Arc::new(Semaphore::new(*v))))
                    .collect(),
            ),
        }
    }

    pub async fn acquire(&self, source_name: Option<&str>) -> ConcurrencyPermit {
        // Semaphore acquire should never fail unless the semaphore is closed,
        // which should never happen in normal operation. If it does, it's a bug.
        let global = self.global.clone().acquire_owned().await.unwrap_or_else(|_| {
            panic!("concurrency semaphore closed unexpectedly. This is a bug - please report it.");
        });
        let source = match source_name {
            Some(src) => self
                .per_source
                .get(src)
                .map(|sem| sem.clone().acquire_owned()),
            None => None,
        };
        let source = match source {
            Some(fut) => Some(fut.await.unwrap_or_else(|_| {
                panic!("per-source concurrency semaphore closed unexpectedly. This is a bug - please report it.");
            })),
            None => None,
        };
        ConcurrencyPermit {
            _global: global,
            _source: source,
        }
    }
}

pub struct ConcurrencyPermit {
    _global: OwnedSemaphorePermit,
    _source: Option<OwnedSemaphorePermit>,
}


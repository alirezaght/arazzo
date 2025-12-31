use std::collections::BTreeMap;
use std::time::Duration;

use arazzo_exec::executor::concurrency::ConcurrencyLimits;

#[tokio::test]
async fn concurrency_limits_enforce_global_limit() {
    let limits = ConcurrencyLimits::new(2, &BTreeMap::new());

    let permit1 = limits.acquire(None).await;
    let permit2 = limits.acquire(None).await;

    let start = std::time::Instant::now();
    let permit3_fut = limits.acquire(None);
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(permit1);
    let permit3 = permit3_fut.await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(50));
    drop(permit2);
    drop(permit3);
}

#[tokio::test]
async fn concurrency_limits_enforce_per_source_limit() {
    let mut per_source = BTreeMap::new();
    per_source.insert("api1".to_string(), 1);
    let limits = ConcurrencyLimits::new(10, &per_source);

    let permit1 = limits.acquire(Some("api1")).await;

    let start = std::time::Instant::now();
    let permit2_fut = limits.acquire(Some("api1"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(permit1);
    let permit2 = permit2_fut.await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(50));
    drop(permit2);
}

#[tokio::test]
async fn concurrency_limits_allow_unlimited_for_unknown_source() {
    let mut per_source = BTreeMap::new();
    per_source.insert("api1".to_string(), 1);
    let limits = ConcurrencyLimits::new(10, &per_source);

    let permit1 = limits.acquire(Some("api1")).await;
    let permit2 = limits.acquire(Some("api2")).await;
    let permit3 = limits.acquire(Some("api2")).await;

    drop(permit1);
    drop(permit2);
    drop(permit3);
}

#[tokio::test]
async fn concurrency_limits_combine_global_and_per_source() {
    let mut per_source = BTreeMap::new();
    per_source.insert("api1".to_string(), 2);
    let limits = ConcurrencyLimits::new(3, &per_source);

    let permit1 = limits.acquire(Some("api1")).await;
    let permit2 = limits.acquire(Some("api1")).await;
    let permit3 = limits.acquire(None).await;

    let start = std::time::Instant::now();
    let permit4_fut = limits.acquire(Some("api1"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(permit1);
    let permit4 = permit4_fut.await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(50));
    drop(permit2);
    drop(permit3);
    drop(permit4);
}

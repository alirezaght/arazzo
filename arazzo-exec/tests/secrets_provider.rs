use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

use arazzo_exec::secrets::{CompositeProvider, EnvSecretsProvider, FileSecretsProvider, SecretsProvider};
use arazzo_exec::secrets::{SecretError, SecretRef};

#[tokio::test]
async fn env_secrets_provider_reads_from_env() {
    std::env::set_var("TEST_SECRET", "secret-value");
    let provider = EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: None,
    };

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "TEST_SECRET".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await.unwrap();
    assert_eq!(std::str::from_utf8(result.expose_bytes()).unwrap(), "secret-value");

    std::env::remove_var("TEST_SECRET");
}

#[tokio::test]
async fn env_secrets_provider_with_prefix() {
    std::env::set_var("PREFIX_TEST_SECRET", "prefixed-value");
    let provider = EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: Some("PREFIX_".to_string()),
    };

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "TEST_SECRET".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await.unwrap();
    assert_eq!(std::str::from_utf8(result.expose_bytes()).unwrap(), "prefixed-value");

    std::env::remove_var("PREFIX_TEST_SECRET");
}

#[tokio::test]
async fn env_secrets_provider_returns_not_found_for_missing() {
    let provider = EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: None,
    };

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "NONEXISTENT".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await;
    assert!(matches!(result, Err(SecretError::NotFound(_))));
}

#[tokio::test]
async fn env_secrets_provider_ignores_wrong_scheme() {
    let provider = EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: None,
    };

    let secret_ref = SecretRef {
        scheme: "file-secrets".to_string(),
        id: "TEST".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await;
    assert!(matches!(result, Err(SecretError::NotFound(_))));
}

#[tokio::test]
async fn file_secrets_provider_reads_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let secret_file = temp_dir.path().join("my-secret");
    std::fs::write(&secret_file, b"file-secret-value").unwrap();

    let provider = FileSecretsProvider {
        scheme: "file-secrets".to_string(),
        base_dir: temp_dir.path().to_path_buf(),
    };

    let secret_ref = SecretRef {
        scheme: "file-secrets".to_string(),
        id: "my-secret".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await.unwrap();
    assert_eq!(std::str::from_utf8(result.expose_bytes()).unwrap(), "file-secret-value");
}

#[tokio::test]
async fn file_secrets_provider_returns_error_for_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let provider = FileSecretsProvider {
        scheme: "file-secrets".to_string(),
        base_dir: temp_dir.path().to_path_buf(),
    };

    let secret_ref = SecretRef {
        scheme: "file-secrets".to_string(),
        id: "nonexistent".to_string(),
        query: None,
    };

    let result = provider.get(&secret_ref).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn composite_provider_tries_providers_in_order() {
    let temp_dir = TempDir::new().unwrap();
    let secret_file = temp_dir.path().join("composite-secret");
    std::fs::write(&secret_file, b"file-value").unwrap();

    std::env::set_var("ENV_SECRET", "env-value");

    let env_provider = Box::new(EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: None,
    });
    let file_provider = Box::new(FileSecretsProvider {
        scheme: "file-secrets".to_string(),
        base_dir: temp_dir.path().to_path_buf(),
    });

    let composite = CompositeProvider::new(vec![env_provider, file_provider]);

    let secret_ref1 = SecretRef {
        scheme: "secrets".to_string(),
        id: "ENV_SECRET".to_string(),
        query: None,
    };
    let result1 = composite.get(&secret_ref1).await.unwrap();
    assert_eq!(std::str::from_utf8(result1.expose_bytes()).unwrap(), "env-value");

    let secret_ref2 = SecretRef {
        scheme: "file-secrets".to_string(),
        id: "composite-secret".to_string(),
        query: None,
    };
    let result2 = composite.get(&secret_ref2).await.unwrap();
    assert_eq!(std::str::from_utf8(result2.expose_bytes()).unwrap(), "file-value");

    std::env::remove_var("ENV_SECRET");
}

#[tokio::test]
async fn composite_provider_returns_not_found_when_all_fail() {
    let composite = CompositeProvider::new(vec![]);

    let secret_ref = SecretRef {
        scheme: "secrets".to_string(),
        id: "NONEXISTENT".to_string(),
        query: None,
    };

    let result = composite.get(&secret_ref).await;
    assert!(matches!(result, Err(SecretError::NotFound(_))));
}

#[tokio::test]
async fn secrets_provider_get_many() {
    std::env::set_var("SECRET1", "value1");
    std::env::set_var("SECRET2", "value2");

    let provider = EnvSecretsProvider {
        scheme: "secrets".to_string(),
        env_prefix: None,
    };

    let refs = vec![
        SecretRef {
            scheme: "secrets".to_string(),
            id: "SECRET1".to_string(),
            query: None,
        },
        SecretRef {
            scheme: "secrets".to_string(),
            id: "SECRET2".to_string(),
            query: None,
        },
    ];

    let result = provider.get_many(&refs).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(std::str::from_utf8(result[&refs[0]].expose_bytes()).unwrap(), "value1");
    assert_eq!(std::str::from_utf8(result[&refs[1]].expose_bytes()).unwrap(), "value2");

    std::env::remove_var("SECRET1");
    std::env::remove_var("SECRET2");
}


use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_validate_command() {
    let mut cmd = Command::cargo_bin("arazzo").unwrap();

    // Create a minimal valid workflow
    let workflow = r#"
arazzo: 1.0.1
info:
  title: Test
  version: 1.0.0
sourceDescriptions:
  - name: api
    type: openapi
    url: https://example.com/openapi.json
workflows:
  - workflowId: test
    steps:
      - stepId: step1
        operationId: getUsers
"#;

    let tmp_dir = TempDir::new().unwrap();
    let workflow_path = tmp_dir.path().join("test.yaml");
    fs::write(&workflow_path, workflow).unwrap();

    cmd.args(&["validate", workflow_path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_validate_invalid_workflow() {
    let mut cmd = Command::cargo_bin("arazzo").unwrap();

    let tmp_dir = TempDir::new().unwrap();
    let workflow_path = tmp_dir.path().join("invalid.yaml");
    fs::write(&workflow_path, "invalid: yaml: content").unwrap();

    cmd.args(&["validate", workflow_path.to_str().unwrap()])
        .assert()
        .failure()
        .code(2); // VALIDATION_FAILED
}

#[test]
fn test_plan_command() {
    let mut cmd = Command::cargo_bin("arazzo").unwrap();

    let workflow = r#"
arazzo: 1.0.1
info:
  title: Test
  version: 1.0.0
sourceDescriptions:
  - name: api
    type: openapi
    url: https://example.com/openapi.json
workflows:
  - workflowId: test
    steps:
      - stepId: step1
        operationId: getUsers
"#;

    let tmp_dir = TempDir::new().unwrap();
    let workflow_path = tmp_dir.path().join("test.yaml");
    fs::write(&workflow_path, workflow).unwrap();

    cmd.args(&["plan", workflow_path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_plan_dot_format() {
    let mut cmd = Command::cargo_bin("arazzo").unwrap();

    // Use a simple workflow that doesn't require OpenAPI resolution
    let workflow = r#"
arazzo: 1.0.1
info:
  title: Test
  version: 1.0.0
sourceDescriptions:
  - name: api
    type: openapi
    url: https://example.com/openapi.json
workflows:
  - workflowId: test
    steps:
      - stepId: step1
        operationId: getUsers
"#;

    let tmp_dir = TempDir::new().unwrap();
    let workflow_path = tmp_dir.path().join("test.yaml");
    fs::write(&workflow_path, workflow).unwrap();

    let assert = cmd
        .args(&["plan", "--format", "dot", workflow_path.to_str().unwrap()])
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph"));
    assert!(stdout.contains("test"));
}

use assert_cmd::Command;
use tempfile::NamedTempFile;

fn write_temp(contents: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().expect("tempfile");
    std::io::Write::write_all(&mut f, contents.as_bytes()).expect("write");
    f
}

#[test]
fn validate_command_returns_0_for_valid_doc() {
    let doc = r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: petStoreDescription
    url: https://example.com/openapi.yaml
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: op1
"#;
    let f = write_temp(doc);

    let bin = assert_cmd::cargo::cargo_bin!("arazzo-cli");
    Command::new(bin)
        .args(["validate", f.path().to_string_lossy().as_ref()])
        .assert()
        .success();
}

#[test]
fn validate_command_returns_1_for_invalid_doc() {
    let doc = r#"
arazzo: 2.0.0
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: petStoreDescription
    url: https://example.com/openapi.yaml
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: op1
"#;
    let f = write_temp(doc);

    let bin = assert_cmd::cargo::cargo_bin!("arazzo-cli");
    Command::new(bin)
        .args(["validate", f.path().to_string_lossy().as_ref()])
        .assert()
        .code(2); // VALIDATION_FAILED
}

#[test]
fn plan_command_outputs_json() {
    let doc = r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: storeApi
    url: https://example.com/openapi.yaml
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: op1
"#;
    let f = write_temp(doc);

    let bin = assert_cmd::cargo::cargo_bin!("arazzo-cli");
    Command::new(bin)
        .args([
            "plan",
            f.path().to_string_lossy().as_ref(),
            "--workflow",
            "w1",
            "--format",
            "json",
        ])
        .assert()
        .success();
}

#[test]
fn plan_command_can_compile_against_local_openapi() {
    // Minimal OpenAPI with an operationId and a required header param + requestBody.
    let openapi = r#"
openapi: 3.0.0
info:
  title: Store API
  version: 1.0.0
paths:
  /orders:
    post:
      operationId: createOrder
      parameters:
        - name: X-Api-Key
          in: header
          required: true
          schema:
            type: string
      requestBody:
        required: true
        content:
          application/json: {}
      responses:
        "200":
          description: ok
"#;
    let openapi_file = write_temp(openapi);

    let arazzo = format!(
        r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: storeApi
    url: {}
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: createOrder
        parameters:
          - name: X-Api-Key
            in: header
            value: "k"
        requestBody:
          contentType: application/json
          payload:
            foo: bar
"#,
        openapi_file.path().to_string_lossy()
    );
    let arazzo_file = write_temp(&arazzo);

    let bin = assert_cmd::cargo::cargo_bin!("arazzo-cli");
    Command::new(bin)
        .args([
            "plan",
            arazzo_file.path().to_string_lossy().as_ref(),
            "--workflow",
            "w1",
            "--format",
            "json",
            "--compile",
        ])
        .assert()
        .success();
}


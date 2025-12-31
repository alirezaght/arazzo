use arazzo_core::{parse_document_str, validate_document, DocumentFormat};

fn minimal_valid_yaml() -> &'static str {
    r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: petStoreDescription
    url: https://example.com/openapi.yaml
    type: openapi
workflows:
  - workflowId: loginUser
    steps:
      - stepId: loginStep
        operationId: loginUser
"#
}

#[test]
fn parse_yaml_and_validate_ok() {
    let parsed = parse_document_str(minimal_valid_yaml(), DocumentFormat::Yaml).unwrap();
    validate_document(&parsed.document).unwrap();
}

#[test]
fn parse_auto_detects_yaml() {
    let parsed = parse_document_str(minimal_valid_yaml(), DocumentFormat::Auto).unwrap();
    assert_eq!(parsed.format, DocumentFormat::Yaml);
}

#[test]
fn parse_json_and_validate_ok() {
    let json = r#"
{
  "arazzo": "1.0.1",
  "info": { "title": "Example", "version": "0.0.1" },
  "sourceDescriptions": [
    { "name": "petStoreDescription", "url": "https://example.com/openapi.yaml", "type": "openapi" }
  ],
  "workflows": [
    {
      "workflowId": "loginUser",
      "steps": [
        { "stepId": "loginStep", "operationId": "loginUser" }
      ]
    }
  ]
}
"#;
    let parsed = parse_document_str(json, DocumentFormat::Json).unwrap();
    validate_document(&parsed.document).unwrap();
}

#[test]
fn parse_auto_detects_json() {
    let json = r#"{ "arazzo": "1.0.1", "info": { "title": "Example", "version": "0.0.1" }, "sourceDescriptions": [ { "name": "src1", "url": "https://example.com/openapi.yaml" } ], "workflows": [ { "workflowId": "w1", "steps": [ { "stepId": "s1", "operationId": "op1" } ] } ] }"#;
    let parsed = parse_document_str(json, DocumentFormat::Auto).unwrap();
    assert_eq!(parsed.format, DocumentFormat::Json);
}

#[test]
fn parse_unknown_format_is_rejected() {
    let err = parse_document_str("not: [valid", DocumentFormat::Auto).unwrap_err();
    // The parser now returns specific YAML/JSON errors instead of generic "unknown format"
    // Since "not: [valid" looks like YAML, it will try YAML first and return a YAML parsing error
    assert!(
        format!("{err}").contains("failed to parse as YAML") || format!("{err}").contains("YAML")
    );
}

#[test]
fn invalid_spec_version_is_rejected() {
    let bad = minimal_valid_yaml().replace("arazzo: 1.0.1", "arazzo: 2.0.0");
    let parsed = parse_document_str(&bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err.violations.iter().any(|v| v.path == "$.arazzo"));
}

#[test]
fn duplicate_workflow_ids_are_rejected() {
    let bad = r#"
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
  - workflowId: w1
    steps:
      - stepId: s2
        operationId: op2
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.message.contains("must be unique")));
}

#[test]
fn step_must_target_exactly_one_of_operation_or_workflow() {
    let bad = r#"
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
        workflowId: otherWorkflow
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err.violations.iter().any(|v| v.path.ends_with(".steps[0]")));
}

#[test]
fn operation_step_parameters_require_in() {
    let bad = r#"
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
        parameters:
          - name: q
            value: 1
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.path.ends_with(".parameters[0].in")));
}

#[test]
fn goto_success_action_requires_step_or_workflow_id() {
    let bad = r#"
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
        onSuccess:
          - name: next
            type: goto
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.message.contains("type=goto")));
}

#[test]
fn jsonpath_criterion_requires_context() {
    let bad = r#"
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
        successCriteria:
          - condition: $[?count(@.pets) > 0]
            type: jsonpath
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.path.ends_with(".successCriteria[0].context")));
}

#[test]
fn components_keys_must_match_regex() {
    let bad = r#"
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
components:
  parameters:
    "bad key!":
      name: q
      in: query
      value: 1
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.message.contains("map key must match")));
}

#[test]
fn invalid_runtime_expression_in_step_outputs_is_rejected() {
    let bad = r#"
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
        outputs:
          x: $inputs..bad
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.path.ends_with(".steps[0].outputs.x")
            && v.message.contains("invalid runtime expression")));
}

#[test]
fn invalid_template_expression_in_operation_path_is_rejected() {
    let bad = r#"
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
        operationPath: '{$sourceDescriptions..url}#/paths/~1pets/get'
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.path.ends_with(".steps[0].operationPath")
            && v.message.contains("invalid template expression")));
}

#[test]
fn invalid_embedded_expression_in_request_body_payload_is_rejected() {
    let bad = r#"
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
        requestBody:
          contentType: application/json
          payload: '{\"petId\": \"{$inputs..pet_id}\"}'
"#;
    let parsed = parse_document_str(bad, DocumentFormat::Yaml).unwrap();
    let err = validate_document(&parsed.document).unwrap_err();
    assert!(err
        .violations
        .iter()
        .any(|v| v.path.ends_with(".steps[0].requestBody.payload")
            && v.message.contains("invalid expression inside value")));
}

use std::io::Write;

use arazzo_core::{DocumentFormat, parse_document_str};
use arazzo_exec::{Compiler};

fn write_temp(contents: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    f.write_all(contents.as_bytes()).expect("write");
    f
}

#[tokio::test]
async fn supports_ref_for_parameters_and_request_body() {
    let openapi = r#"
openapi: 3.0.0
info:
  title: Store API
  version: 1.0.0
components:
  parameters:
    ApiKey:
      name: X-Api-Key
      in: header
      required: true
      schema:
        type: string
  requestBodies:
    CreateOrder:
      required: true
      content:
        application/json: {}
paths:
  /orders:
    post:
      operationId: createOrder
      parameters:
        - $ref: '#/components/parameters/ApiKey'
      requestBody:
        $ref: '#/components/requestBodies/CreateOrder'
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

    let doc = parse_document_str(&arazzo, DocumentFormat::Yaml).unwrap().document;
    let wf = &doc.workflows[0];

    let compiled = Compiler::default().compile_workflow(&doc, wf).await;
    assert!(compiled.diagnostics.is_empty(), "unexpected top-level diagnostics: {:?}", compiled.diagnostics);

    let step = &compiled.steps[0];
    assert!(step.diagnostics.is_empty(), "unexpected step diagnostics: {:?}", step.diagnostics);
    assert!(step.missing_required_parameters.is_empty());
    assert!(!step.missing_required_request_body);

    let op = step.operation.as_ref().expect("operation resolved");
    assert_eq!(op.method, "POST");
    assert_eq!(op.path, "/orders");
    assert_eq!(op.shape.request_body_required, Some(true));
    assert_eq!(
        op.shape.request_body_content_types.as_ref().unwrap(),
        &vec!["application/json".to_string()]
    );
}

#[tokio::test]
async fn unqualified_operation_id_is_ambiguous_across_sources() {
    let openapi_a = r#"
openapi: 3.0.0
info: { title: A, version: 1.0.0 }
paths:
  /a:
    get:
      operationId: op1
      responses: { "200": { description: ok } }
"#;
    let openapi_b = r#"
openapi: 3.0.0
info: { title: B, version: 1.0.0 }
paths:
  /b:
    get:
      operationId: op1
      responses: { "200": { description: ok } }
"#;
    let fa = write_temp(openapi_a);
    let fb = write_temp(openapi_b);

    let arazzo = format!(
        r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: a
    url: {}
  - name: b
    url: {}
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: op1
"#,
        fa.path().to_string_lossy(),
        fb.path().to_string_lossy(),
    );

    let doc = parse_document_str(&arazzo, DocumentFormat::Yaml).unwrap().document;
    let wf = &doc.workflows[0];

    let compiled = Compiler::default().compile_workflow(&doc, wf).await;
    let step = &compiled.steps[0];
    assert!(step.operation.is_none());
    assert!(
        step.diagnostics.iter().any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Error
            && d.message.contains("ambiguous operationId 'op1'")),
        "expected ambiguity error, got: {:?}",
        step.diagnostics
    );
}

#[tokio::test]
async fn unqualified_operation_id_is_resolved_when_unique_and_warns() {
    let openapi_a = r#"
openapi: 3.0.0
info: { title: A, version: 1.0.0 }
paths:
  /a:
    get:
      operationId: op1
      responses: { "200": { description: ok } }
"#;
    let openapi_b = r#"
openapi: 3.0.0
info: { title: B, version: 1.0.0 }
paths:
  /b:
    get:
      operationId: other
      responses: { "200": { description: ok } }
"#;
    let fa = write_temp(openapi_a);
    let fb = write_temp(openapi_b);

    let arazzo = format!(
        r#"
arazzo: 1.0.1
info:
  title: Example
  version: 0.0.1
sourceDescriptions:
  - name: a
    url: {}
  - name: b
    url: {}
workflows:
  - workflowId: w1
    steps:
      - stepId: s1
        operationId: op1
"#,
        fa.path().to_string_lossy(),
        fb.path().to_string_lossy(),
    );

    let doc = parse_document_str(&arazzo, DocumentFormat::Yaml).unwrap().document;
    let wf = &doc.workflows[0];

    let compiled = Compiler::default().compile_workflow(&doc, wf).await;
    let step = &compiled.steps[0];
    let op = step.operation.as_ref().expect("operation resolved");
    assert_eq!(op.source_name, "a");

    assert!(
        step.diagnostics.iter().any(|d| d.severity == arazzo_exec::openapi::DiagnosticSeverity::Warning
            && d.message.contains("unqualified operationId 'op1' resolved to source 'a'")),
        "expected warning, got: {:?}",
        step.diagnostics
    );
}


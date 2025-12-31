use arazzo_core::{plan_from_str, DocumentFormat, PlanOptions};

#[test]
fn planner_builds_levels_from_step_data_dependencies() {
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
      - stepId: login
        operationId: loginUser
        outputs:
          token: $response.body#/token
      - stepId: createOrder
        operationId: createOrder
        parameters:
          - name: Authorization
            in: header
            value: $steps.login.outputs.token
        outputs:
          orderId: $response.body#/id
      - stepId: fetchOrder
        operationId: fetchOrder
        parameters:
          - name: id
            in: path
            value: $steps.createOrder.outputs.orderId
"#;

    let outcome = plan_from_str(
        doc,
        DocumentFormat::Yaml,
        PlanOptions {
            workflow_id: Some("w1".to_string()),
            inputs: None,
        },
    )
    .unwrap();

    assert!(outcome.validation.is_valid);
    let plan = outcome.plan.unwrap();
    assert_eq!(
        plan.graph.levels,
        vec![
            vec!["login".to_string()],
            vec!["createOrder".to_string()],
            vec!["fetchOrder".to_string()],
        ]
    );
    assert_eq!(
        plan.graph.topo_order,
        vec![
            "login".to_string(),
            "createOrder".to_string(),
            "fetchOrder".to_string(),
        ]
    );
}

#[test]
fn planner_detects_parallel_steps_in_same_level() {
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
      - stepId: login
        operationId: loginUser
        outputs:
          token: $response.body#/token
      - stepId: a
        operationId: opA
        parameters:
          - name: Authorization
            in: header
            value: $steps.login.outputs.token
      - stepId: b
        operationId: opB
        parameters:
          - name: Authorization
            in: header
            value: $steps.login.outputs.token
"#;

    let outcome = plan_from_str(
        doc,
        DocumentFormat::Yaml,
        PlanOptions {
            workflow_id: Some("w1".to_string()),
            inputs: None,
        },
    )
    .unwrap();

    let plan = outcome.plan.unwrap();
    assert_eq!(plan.graph.levels[0], vec!["login".to_string()]);
    assert_eq!(plan.graph.levels[1], vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn planner_reports_missing_inputs_when_inputs_json_not_provided() {
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
        parameters:
          - name: q
            in: query
            value: $inputs.userId
"#;

    let outcome = plan_from_str(
        doc,
        DocumentFormat::Yaml,
        PlanOptions {
            workflow_id: Some("w1".to_string()),
            inputs: None,
        },
    )
    .unwrap();

    let plan = outcome.plan.unwrap();
    assert!(plan.summary.missing_inputs.contains("userId"));
}

#[test]
fn planner_considers_inputs_json_satisfied() {
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
        parameters:
          - name: q
            in: query
            value: $inputs.userId
"#;

    let outcome = plan_from_str(
        doc,
        DocumentFormat::Yaml,
        PlanOptions {
            workflow_id: Some("w1".to_string()),
            inputs: Some(serde_json::json!({"userId": 123})),
        },
    )
    .unwrap();

    let plan = outcome.plan.unwrap();
    assert!(!plan.summary.missing_inputs.contains("userId"));
}

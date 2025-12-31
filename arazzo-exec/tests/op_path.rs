use arazzo_exec::openapi::op_path::parse_operation_path_ref;

#[test]
fn parse_operation_path_ref_valid() {
    let result = parse_operation_path_ref(
        "{$sourceDescriptions.petStoreDescription.url}#/paths/~1pet~1findByStatus/get",
    )
    .unwrap();

    assert_eq!(result.0, "petStoreDescription");
    assert_eq!(result.1, "/paths/~1pet~1findByStatus/get");
    assert_eq!(result.2, "get");
    assert_eq!(result.3, "/pet/findByStatus");
}

#[test]
fn parse_operation_path_ref_missing_hash() {
    let result = parse_operation_path_ref("{$sourceDescriptions.petStoreDescription.url}");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("operationPath must include"));
}

#[test]
fn parse_operation_path_ref_missing_source_template() {
    let result = parse_operation_path_ref("https://example.com#/paths/~1pet/get");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("operationPath must contain"));
}

#[test]
fn parse_operation_path_ref_invalid_pointer() {
    let result =
        parse_operation_path_ref("{$sourceDescriptions.petStoreDescription.url}#/invalid/path");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("operationPath pointer must point"));
}

#[test]
fn parse_operation_path_ref_different_methods() {
    let methods = vec!["get", "post", "put", "delete", "patch"];
    for method in methods {
        let path = format!(
            "{{$sourceDescriptions.api.url}}#/paths/~1resource/{}",
            method
        );
        let result = parse_operation_path_ref(&path).unwrap();
        assert_eq!(result.2, method);
    }
}

#[test]
fn parse_operation_path_ref_complex_path() {
    let result = parse_operation_path_ref(
        "{$sourceDescriptions.api.url}#/paths/~1users~1{userId}~1orders/get",
    )
    .unwrap();

    assert_eq!(result.3, "/users/{userId}/orders");
}

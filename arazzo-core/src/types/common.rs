use std::collections::BTreeMap;

pub type AnyValue = serde_json::Value;
pub type JsonSchema = serde_json::Value;
pub type RuntimeExpression = String;

/// Specification Extensions (`x-...`) captured from the document.
///
/// We deserialize "extra" fields into this map and validate the `x-` prefix at validation time.
pub type Extensions = BTreeMap<String, serde_json::Value>;

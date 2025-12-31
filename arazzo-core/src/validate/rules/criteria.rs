use crate::types::{
    Criterion, CriterionExpressionLanguage, CriterionType, KnownCriterionType,
};
use crate::validate::rules::common::validate_runtime_expr;
use crate::validate::validator::Validator;

pub(crate) fn validate_criteria_list(v: &mut Validator, path: &str, criteria: &[Criterion]) {
    for (idx, c) in criteria.iter().enumerate() {
        let ipath = format!("{path}[{idx}]");
        v.validate_extensions(&ipath, &c.extensions);

        if c.condition.trim().is_empty() {
            v.push(format!("{ipath}.condition"), "must not be empty");
        }

        let requires_context = match c.r#type.as_ref() {
            None => false,
            Some(CriterionType::Known(KnownCriterionType::Simple)) => false,
            Some(_) => true,
        };

        if requires_context && c.context.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            v.push(
                format!("{ipath}.context"),
                "must be provided when type is regex/jsonpath/xpath/custom",
            );
        }

        if let Some(ctx) = &c.context {
            validate_runtime_expr(v, &format!("{ipath}.context"), ctx);
        }

        if let Some(CriterionType::Custom(custom)) = &c.r#type {
            v.validate_extensions(&format!("{ipath}.type"), &custom.extensions);
            match custom.r#type {
                CriterionExpressionLanguage::Jsonpath => {
                    if custom.version != "draft-goessner-dispatch-jsonpath-00" {
                        v.push(
                            format!("{ipath}.type.version"),
                            "unsupported jsonpath version (expected draft-goessner-dispatch-jsonpath-00)",
                        );
                    }
                }
                CriterionExpressionLanguage::Xpath => {
                    let allowed = ["xpath-30", "xpath-20", "xpath-10"];
                    if !allowed.contains(&custom.version.as_str()) {
                        v.push(
                            format!("{ipath}.type.version"),
                            "unsupported xpath version (expected xpath-30, xpath-20, or xpath-10)",
                        );
                    }
                }
            }
        }
    }
}


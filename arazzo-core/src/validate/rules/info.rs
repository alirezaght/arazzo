use crate::types::Info;
use crate::validate::validator::Validator;

pub(crate) fn validate_info(v: &mut Validator, info: &Info, path: &str) {
    v.validate_extensions(path, &info.extensions);

    if info.title.trim().is_empty() {
        v.push(format!("{path}.title"), "must not be empty");
    }
    if info.version.trim().is_empty() {
        v.push(format!("{path}.version"), "must not be empty");
    }
}

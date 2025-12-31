mod rules;
mod validator;

use crate::error::ValidationError;
use crate::types::ArazzoDocument;
use validator::Validator;

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

impl Validate for ArazzoDocument {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_document(self)
    }
}

pub fn validate_document(doc: &ArazzoDocument) -> Result<(), ValidationError> {
    let mut v = Validator::new();
    v.validate_document(doc);
    v.finish()
}

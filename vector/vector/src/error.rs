use crate::exports::golem::vector::types::VectorError;

impl<'a> From<&'a VectorError> for VectorError {
    fn from(value: &'a VectorError) -> Self {
        value.clone()
    }
}

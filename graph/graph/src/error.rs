use crate::golem::graph::errors::GraphError;
use crate::golem::graph::types::ElementId;

pub mod mapping {
    use super::*;

    /// Extract element ID from error message or error body
    pub fn extract_element_id_from_message(message: &str) -> Option<ElementId> {
        // Looking for patterns like "collection/key" or just "key"
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z0-9_]+/[a-zA-Z0-9_-]+)") {
            if let Some(captures) = re.captures(message) {
                if let Some(matched) = captures.get(1) {
                    return Some(ElementId::StringValue(matched.as_str().to_string()));
                }
            }
        }

        // Look for quoted strings that might be IDs
        if let Ok(re) = regex::Regex::new(r#""([^"]+)""#) {
            if let Some(captures) = re.captures(message) {
                if let Some(matched) = captures.get(1) {
                    let id_str = matched.as_str();
                    if id_str.contains('/') || id_str.len() > 3 {
                        return Some(ElementId::StringValue(id_str.to_string()));
                    }
                }
            }
        }
        None
    }
}

impl<'a> From<&'a GraphError> for GraphError {
    fn from(e: &'a GraphError) -> GraphError {
        e.clone()
    }
}

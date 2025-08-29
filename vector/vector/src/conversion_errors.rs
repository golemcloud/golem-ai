//! Enhanced error types for vector conversion operations.

use crate::exports::golem::vector::types::VectorError;

/// Specialized conversion errors with better context
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Invalid vector format: {0}")]
    InvalidVector(String),

    #[error("Unsupported metadata type: {0}")]
    UnsupportedMetadata(String),

    #[error("Filter translation failed: {0}")]
    FilterTranslation(String),

    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Invalid metadata value for field '{field}': {reason}")]
    InvalidMetadataValue { field: String, reason: String },

    #[error("Unsupported filter operator: {operator} for provider {provider}")]
    UnsupportedFilterOperator { operator: String, provider: String },

    #[error("Distance metric {metric} not supported by provider {provider}")]
    UnsupportedMetric { metric: String, provider: String },

    #[error("Filter nesting depth {depth} exceeds maximum {max} for provider {provider}")]
    FilterNestingTooDeep {
        depth: usize,
        max: usize,
        provider: String,
    },

    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

impl From<ConversionError> for VectorError {
    fn from(e: ConversionError) -> Self {
        match e {
            ConversionError::InvalidVector(_) => VectorError::InvalidVector(e.to_string()),
            ConversionError::DimensionMismatch { .. } => {
                VectorError::DimensionMismatch(e.to_string())
            }
            ConversionError::InvalidMetadataValue { .. } | ConversionError::ValidationFailed(_) => {
                VectorError::InvalidParams(e.to_string())
            }
            ConversionError::UnsupportedMetadata(_)
            | ConversionError::UnsupportedFilterOperator { .. }
            | ConversionError::UnsupportedMetric { .. }
            | ConversionError::FilterNestingTooDeep { .. } => {
                VectorError::UnsupportedFeature(e.to_string())
            }
            ConversionError::FilterTranslation(_) => VectorError::ProviderError(e.to_string()),
        }
    }
}

/// Trait for validating conversions before they happen
pub trait ConversionValidator<T> {
    fn validate(&self, input: &T) -> Result<(), ConversionError>;
}

/// Helper function for validating vector dimensions
pub fn validate_vector_dimension(
    vector: &[f32],
    expected_dim: Option<usize>,
) -> Result<(), ConversionError> {
    if let Some(expected) = expected_dim {
        if vector.len() != expected {
            return Err(ConversionError::DimensionMismatch {
                expected,
                actual: vector.len(),
            });
        }
    }

    if vector.is_empty() {
        return Err(ConversionError::InvalidVector(
            "Vector cannot be empty".to_string(),
        ));
    }

    // Check for invalid values
    for (i, &val) in vector.iter().enumerate() {
        if !val.is_finite() {
            return Err(ConversionError::InvalidVector(format!(
                "Vector contains non-finite value at index {i}: {val}"
            )));
        }
    }

    Ok(())
}

/// Helper function for validating filter expression depth
pub fn validate_filter_depth<T>(
    expr: &T,
    current_depth: usize,
    max_depth: usize,
    provider: &str,
    depth_fn: impl Fn(&T) -> Vec<&T>,
) -> Result<(), ConversionError> {
    if current_depth > max_depth {
        return Err(ConversionError::FilterNestingTooDeep {
            depth: current_depth,
            max: max_depth,
            provider: provider.to_string(),
        });
    }

    for child in depth_fn(expr) {
        validate_filter_depth(child, current_depth + 1, max_depth, provider, &depth_fn)?;
    }

    Ok(())
}

use crate::golem::vector::types::VectorError;
use std::ffi::OsStr;

pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(VectorError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = VectorError::ProviderError(format!("Missing config key: {key_str}"));
            fail(error)
        }
    }
}

pub fn get_optional_config(key: impl AsRef<OsStr>) -> Option<String> {
    std::env::var(key).ok()
}

pub fn get_config_with_default(key: impl AsRef<OsStr>, default: impl Into<String>) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

pub fn validate_config_key(key: impl AsRef<OsStr>) -> Result<String, VectorError> {
    let key_str = key.as_ref().to_string_lossy().to_string();
    std::env::var(key)
        .map_err(|_| VectorError::ProviderError(format!("Missing config key: {key_str}")))
}

pub fn with_config_keys<R>(keys: &[&str], callback: impl FnOnce(Vec<String>) -> R) -> R {
    let mut values = Vec::new();
    for key in keys {
        match std::env::var(key) {
            Ok(value) => values.push(value),
            Err(_) => {
                return callback(Vec::new());
            }
        }
    }
    callback(values)
}

pub fn with_connection_config_key(
    metadata: &Option<crate::golem::vector::types::Metadata>,
    key: &str,
) -> Option<String> {
    if let Some(metadata_list) = metadata {
        for (k, v) in metadata_list {
            if k == key {
                return match v {
                    crate::golem::vector::types::MetadataValue::StringVal(s) => Some(s.clone()),
                    crate::golem::vector::types::MetadataValue::NumberVal(n) => Some(n.to_string()),
                    crate::golem::vector::types::MetadataValue::IntegerVal(i) => {
                        Some(i.to_string())
                    }
                    crate::golem::vector::types::MetadataValue::BooleanVal(b) => {
                        Some(b.to_string())
                    }
                    _ => None,
                };
            }
        }
    }

    std::env::var(key).ok()
}

pub fn get_timeout_config() -> u64 {
    get_config_with_default("VECTOR_PROVIDER_TIMEOUT", "30")
        .parse()
        .unwrap_or(30)
}

pub fn get_max_retries_config() -> u32 {
    get_config_with_default("VECTOR_PROVIDER_MAX_RETRIES", "3")
        .parse()
        .unwrap_or(3)
}

pub fn get_batch_size_config() -> u32 {
    get_config_with_default("VECTOR_BATCH_SIZE", "100")
        .parse()
        .unwrap_or(100)
}

pub fn get_vector_dimension_config() -> Option<u32> {
    get_optional_config("VECTOR_DIMENSION").and_then(|s| s.parse().ok())
}

pub fn get_provider_config(provider: &str) -> std::collections::HashMap<String, String> {
    let mut config = std::collections::HashMap::new();

    if let Some(api_key) = get_optional_config(format!("{}_API_KEY", provider.to_uppercase())) {
        config.insert("api_key".to_string(), api_key);
    }

    if let Some(endpoint) = get_optional_config(format!("{}_ENDPOINT", provider.to_uppercase())) {
        config.insert("endpoint".to_string(), endpoint);
    }

    if let Some(region) = get_optional_config(format!("{}_REGION", provider.to_uppercase())) {
        config.insert("region".to_string(), region);
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_config_with_default() {
        let result = get_config_with_default("NONEXISTENT_CONFIG_KEY", "default_value");
        assert_eq!(result, "default_value");
    }

    #[test]
    fn test_get_optional_config() {
        let result = get_optional_config("NONEXISTENT_CONFIG_KEY");
        assert!(result.is_none());
    }

    #[test]
    fn test_timeout_config() {
        env::set_var("VECTOR_PROVIDER_TIMEOUT", "60");
        assert_eq!(get_timeout_config(), 60);
        env::remove_var("VECTOR_PROVIDER_TIMEOUT");
        assert_eq!(get_timeout_config(), 30); // default
    }

    #[test]
    fn test_get_provider_config() {
        env::set_var("PINECONE_API_KEY", "test_key");
        env::set_var("PINECONE_ENDPOINT", "test_endpoint");

        let config = get_provider_config("pinecone");
        assert_eq!(config.get("api_key"), Some(&"test_key".to_string()));
        assert_eq!(config.get("endpoint"), Some(&"test_endpoint".to_string()));

        env::remove_var("PINECONE_API_KEY");
        env::remove_var("PINECONE_ENDPOINT");
    }
}

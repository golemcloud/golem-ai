//! Authentication for Google Cloud TTS
//!
//! Supports API Key and provides hook for OAuth2.

use crate::types;

/// Get authentication key (API Key)
pub fn get_auth_key() -> Result<String, crate::bindings::exports::golem::tts::types::TtsError> {
    match std::env::var("GOOGLE_API_KEY") {
        Ok(key) => {
            if key.is_empty() {
                return Err(types::unauthorized_error("GOOGLE_API_KEY is empty"));
            }
            Ok(key)
        }
        Err(_) => {
            // Check for GOOGLE_APPLICATION_CREDENTIALS as a fallback (not fully implemented yet)
            if std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok() {
                return Err(types::unauthorized_error("Service Account JSON provided but OAuth2 token exchange not yet implemented. Please use GOOGLE_API_KEY for now."));
            }
            Err(types::unauthorized_error(
                "GOOGLE_API_KEY environment variable not set",
            ))
        }
    }
}

pub mod api;
mod gcp_auth;
mod gcp_cloud_storage;
mod gcp_speech_to_text;
pub mod request;
pub mod wasi;

pub use gcp_auth::ServiceAccountKey;
pub use gcp_cloud_storage::CloudStorageClient;
pub use gcp_speech_to_text::SpeechToTextClient;

use std::sync::Arc;

use golem_stt::error::Error as SttError;
use golem_stt::{http::WstdHttpClient, runtime::WasiAsyncRuntime};

use crate::transcription::{gcp_auth::GcpAuth, ServiceAccountKey};

use super::{
    api::SpeechToTextApi, gcp_cloud_storage::CloudStorageClient,
    gcp_speech_to_text::SpeechToTextClient,
};

impl
    SpeechToTextApi<
        CloudStorageClient<WstdHttpClient>,
        SpeechToTextClient<WstdHttpClient, WasiAsyncRuntime>,
    >
{
    pub fn live(
        bucket_name: String,
        service_acc_key: ServiceAccountKey,
        location: String,
    ) -> Result<Self, SttError> {
        let gcp_auth = GcpAuth::new(service_acc_key, WstdHttpClient::default()).map_err(|err| {
            SttError::AuthError(format!("failed to create GcpAuth client, {err}"))
        })?;

        let gcp_auth_arc = Arc::new(gcp_auth);

        let cloud_storage_service =
            CloudStorageClient::new(gcp_auth_arc.clone(), WstdHttpClient::default());
        let speech_to_text_service = SpeechToTextClient::new(
            gcp_auth_arc.clone(),
            WstdHttpClient::default(),
            location,
            WasiAsyncRuntime::default(),
        );

        Ok(Self::new(
            bucket_name,
            cloud_storage_service,
            speech_to_text_service,
        ))
    }
}

#[allow(async_fn_in_trait)]
pub trait SttProviderClient<REQ, RES, ERR: std::error::Error> {
    async fn transcribe_audio(&self, request: REQ) -> Result<RES, ERR>;
}

#[cfg(feature = "durability")]
pub mod durable_impl {
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::value_and_type::IntoValue;
    use crate::golem::stt::{transcription, types};

    pub fn persist_transcribe<V: IntoValue + core::fmt::Debug>(
        component_id: &'static str,
        input: V,
        result: Result<transcription::TranscriptionResult, types::SttError>,
    ) -> Result<transcription::TranscriptionResult, types::SttError> {
        type OkT = transcription::TranscriptionResult;
        type ErrT = types::SttError;
        let durability: Durability<OkT, ErrT> = Durability::new(component_id, "transcribe", DurableFunctionType::WriteRemote);
        if durability.is_live() {
            match result {
                Ok(ok) => Ok(durability.persist_infallible(input, ok)),
                Err(e) => Err(e),
            }
        } else {
            Ok(durability.replay_infallible())
        }
    }
}

#[cfg(not(feature = "durability"))]
pub mod durable_impl {
    use golem_rust::value_and_type::IntoValue;
    use crate::golem::stt::{transcription, types};
    pub fn persist_transcribe<V: IntoValue + core::fmt::Debug>(
        _component_id: &'static str,
        _input: V,
        result: Result<transcription::TranscriptionResult, types::SttError>,
    ) -> Result<transcription::TranscriptionResult, types::SttError> { result }
}

pub mod retry {
    use std::time::Duration;
    pub fn with_retries<F, T, E>(mut f: F, max_retries: u32, base_delay_ms: u64) -> Result<T, E>
    where
        F: FnMut(u32) -> Result<T, E>,
    {
        let mut attempt = 0u32;
        loop {
            match f(attempt) {
                Ok(v) => return Ok(v),
                Err(_e) if attempt < max_retries => {
                    let delay = base_delay_ms.saturating_mul(1 + attempt as u64);
                    std::thread::sleep(Duration::from_millis(delay));
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
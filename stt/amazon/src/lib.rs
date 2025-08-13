use golem_stt::golem::stt::transcription::{Guest as TranscriptionGuest, AudioConfig, TranscribeOptions, TranscriptionResult, TranscribeRequest};
use golem_stt::golem::stt::types::SttError;

pub mod config;
pub mod signer;
pub mod error;
mod transcribe;
mod batch;
#[cfg(feature = "durability")]
mod durability;

pub struct AmazonTranscriptionComponent;

impl TranscriptionGuest for AmazonTranscriptionComponent {
    fn transcribe(
        audio: Vec<u8>,
        config: AudioConfig,
        options: Option<TranscribeOptions>,
    ) -> Result<TranscriptionResult, SttError> {
        let cfg = crate::config::AmazonConfig::load()?;
        crate::batch::transcribe_impl(audio, &cfg, options, config)
    }

    fn multi_transcribe(requests: Vec<TranscribeRequest>) -> Result<Vec<TranscriptionResult>, SttError> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut results = Vec::with_capacity(requests.len());
            for req in requests {
                results.push(Self::transcribe(req.audio, req.config, req.options)?);
            }
            return Ok(results);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::{Arc, Mutex, Condvar};
            use std::thread;
            let cfg = crate::config::AmazonConfig::load()?;
            let max_in_flight = cfg.max_concurrency.max(1);
            let len = requests.len();
            let results: Arc<Mutex<Vec<Option<TranscriptionResult>>>> = Arc::new(Mutex::new(vec![None; len]));
            let first_err: Arc<Mutex<Option<SttError>>> = Arc::new(Mutex::new(None));
            let in_flight = Arc::new((Mutex::new(0usize), Condvar::new()));
            let mut handles = Vec::with_capacity(len);
            for (idx, req) in requests.into_iter().enumerate() {
                let results_cloned = Arc::clone(&results);
                let err_cloned = Arc::clone(&first_err);
                let in_flight_cloned = Arc::clone(&in_flight);
                {
                    let (lock, cvar) = &*in_flight_cloned;
                    let mut count = lock.lock().unwrap();
                    while *count >= max_in_flight { count = cvar.wait(count).unwrap(); }
                    *count += 1;
                }
                handles.push(thread::spawn(move || {
                    let out = Self::transcribe(req.audio, req.config, req.options);
                    match out {
                        Ok(v) => { if let Ok(mut guard) = results_cloned.lock() { guard[idx] = Some(v); } }
                        Err(e) => { if let Ok(mut guard) = err_cloned.lock() { if guard.is_none() { *guard = Some(e); } } }
                    }
                    let (lock, cvar) = &*in_flight_cloned;
                    if let Ok(mut count) = lock.lock() { *count = count.saturating_sub(1); cvar.notify_one(); }
                }));
            }
            for h in handles { let _ = h.join(); }
            if let Ok(guard) = first_err.lock() { if let Some(e) = guard.clone() { return Err(e); } }
            let mut out = Vec::with_capacity(len);
            if let Ok(guard) = results.lock() { for v in guard.iter() { out.push(v.clone().unwrap()); } }
            Ok(out)
        }
    }
}

pub struct AmazonLanguagesComponent;

impl golem_stt::golem::stt::languages::Guest for AmazonLanguagesComponent {
    fn list_languages() -> Result<Vec<golem_stt::golem::stt::languages::LanguageInfo>, SttError> {
        Ok(vec![])
    }
}


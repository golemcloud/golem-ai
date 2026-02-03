use crate::exports::golem::tts::synthesis::{Response, Request as WitRequest};
use crate::exports::golem::tts::voices::Error as TtsError;

pub struct TtsRequest {
    pub text: String,
    pub voice_id: String,
}

impl From<WitRequest> for TtsRequest {
    fn from(req: WitRequest) -> Self {
        Self {
            text: req.text,
            voice_id: req.voice_id,
        }
    }
}

pub trait TtsGuest {
    fn synthesize(req: TtsRequest) -> Result<Response, TtsError>;
}

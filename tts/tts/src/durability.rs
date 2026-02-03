use std::marker::PhantomData;
use golem_rust::durability::Durability;
use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
use crate::exports::golem::tts::synthesis::{Guest as WitSynthesisGuest, Response as WitResponse, Request as WitRequest};
use crate::exports::golem::tts::voices::{Guest as WitVoicesGuest, Voice as WitVoice, Error as WitError};
use crate::guest::{TtsGuest, TtsRequest};
use golem_rust::{FromValueAndType, IntoValue};

pub struct DurableTts<T> {
    _worker: PhantomData<T>,
}

pub trait ExtendedGuest: TtsGuest + WitVoicesGuest + 'static {}

#[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
pub struct EmptyInput;

#[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
pub struct SynthesizeInput {
    pub request: WitRequest,
}

impl<T: ExtendedGuest> DurableTts<T> {
    pub fn list_voices() -> Result<Vec<WitVoice>, WitError> {
        let durability = Durability::<Vec<WitVoice>, WitError>::new(
            "golem-tts",
            "list-voices",
            DurableFunctionType::WriteRemote,
        );

        if durability.is_live() {
            let result = T::list_voices();
            durability.persist(EmptyInput, result.clone());
            result
        } else {
            durability.replay()
        }
    }

    pub fn synthesize(req: WitRequest) -> Result<WitResponse, WitError> {
        let durability = Durability::<WitResponse, WitError>::new(
            "golem-tts",
            "synthesize",
            DurableFunctionType::WriteRemote,
        );

        if durability.is_live() {
            let result = T::synthesize(TtsRequest::from(req.clone()));
            durability.persist(
                SynthesizeInput {
                    request: req,
                },
                result.clone(),
            );
            result
        } else {
            durability.replay()
        }
    }
}

impl From<&WitError> for WitError {
    fn from(error: &WitError) -> Self {
        error.clone()
    }
}

impl<T: ExtendedGuest> WitVoicesGuest for DurableTts<T> {
    fn list_voices() -> Result<Vec<WitVoice>, WitError> {
        Self::list_voices()
    }
}

impl<T: ExtendedGuest> WitSynthesisGuest for DurableTts<T> {
    fn synthesize(req: WitRequest) -> Result<WitResponse, WitError> {
        Self::synthesize(req)
    }
}

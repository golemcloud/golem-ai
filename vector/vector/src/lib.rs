pub mod config;
pub mod durability;
pub mod error;

wit_bindgen::generate!({
    path: "../wit",
    world: "vector-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [
        PartialEq,
        Clone,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue
    ],
    pub_export_macro: true,
});

pub use crate::exports::golem;

pub use __export_vector_library_impl as export_vector;

use crate::exports::golem::vector::types::{self, GuestFilterFunc, GuestMetadataFunc};
use golem_rust::value_and_type::{FromValueAndType, IntoValue, TypeNodeBuilder};
use golem_rust::wasm_rpc::{NodeBuilder, ResourceMode, Uri, WitValueExtractor};

use std::cell::RefCell;
use std::str::FromStr;

const METADATA_FUNC_ID: u64 = 1;
const FILTER_FUNC_ID: u64 = 2;

macro_rules! impl_resource_traits {
    ($ResourceType:ty, $InnerType:ty, $UriString:literal, $TypeIdConstant:ident) => {
        impl Clone for $ResourceType {
            fn clone(&self) -> Self {
                Self::new(self.get::<$InnerType>().clone())
            }
        }

        impl PartialEq for $ResourceType {
            fn eq(&self, other: &Self) -> bool {
                self.get::<$InnerType>() == other.get::<$InnerType>()
            }
        }

        impl IntoValue for $ResourceType {
            fn add_to_builder<B: NodeBuilder>(self, builder: B) -> B::Result {
                builder.handle(
                    Uri {
                        value: $UriString.to_string(),
                    },
                    self.handle() as u64,
                )
            }

            fn add_to_type_builder<B: TypeNodeBuilder>(builder: B) -> B::Result {
                builder.handle(None, None, $TypeIdConstant, ResourceMode::Owned)
            }
        }

        impl FromValueAndType for $ResourceType {
            fn from_extractor<'a, 'b>(
                extractor: &'a impl WitValueExtractor<'a, 'b>,
            ) -> Result<Self, String> {
                <$InnerType>::from_extractor(extractor).map(Self::new)
            }
        }
    };
}

impl_resource_traits!(
    types::MetadataFunc,
    types::MetadataValue,
    "golem:vector/types/metadata-func",
    METADATA_FUNC_ID
);
impl_resource_traits!(
    types::FilterFunc,
    types::FilterExpression,
    "golem:vector/types/filter-func",
    FILTER_FUNC_ID
);

impl GuestMetadataFunc for types::MetadataValue {
    fn get(&self) -> types::MetadataValue {
        self.clone()
    }
}

impl GuestFilterFunc for types::FilterExpression {
    fn get(&self) -> types::FilterExpression {
        self.clone()
    }
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_VECTOR_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    /// This holds the state of our application.
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}

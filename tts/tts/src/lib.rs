pub mod durability;
pub mod guest;
pub mod http;

wit_bindgen::generate!({
    path: "../wit",
    world: "tts-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

pub use __export_tts_library_impl as export_tts;

use golem_stt::golem::stt::languages::{Guest, LanguageInfo};
use golem_stt::golem::stt::types::SttError;

pub struct AmazonLanguages;

impl Guest for AmazonLanguages {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(vec![
            LanguageInfo { code: "en-US".into(), name: "English (United States)".into(), native_name: "English (United States)".into() },
            LanguageInfo { code: "en-GB".into(), name: "English (United Kingdom)".into(), native_name: "English (United Kingdom)".into() },
            LanguageInfo { code: "es-US".into(), name: "Spanish (United States)".into(), native_name: "Español (Estados Unidos)".into() },
        ])
    }
}


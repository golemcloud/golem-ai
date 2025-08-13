use golem_stt::golem::stt::languages::{LanguageInfo, Guest as LanguagesGuest};
use golem_stt::golem::stt::types::SttError;

pub struct DeepgramLanguagesComponent;

impl DeepgramLanguagesComponent {
    pub fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        let langs = vec![
            LanguageInfo { code: "en-US".to_string(), name: "English (US)".to_string(), native_name: "English".to_string() },
        ];
        Ok(langs)
    }
}

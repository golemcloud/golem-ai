use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::types::SttError;

const LANGUAGES: &[(&str, &str, &str)] = &[
    ("en-US", "English (United States)", "English (United States)"),
    ("en-GB", "English (United Kingdom)", "English (United Kingdom)"),
    ("es-ES", "Spanish (Spain)", "Español (España)"),
    ("es-MX", "Spanish (Mexico)", "Español (México)"),
    ("fr-FR", "French (France)", "Français (France)"),
    ("de-DE", "German (Germany)", "Deutsch (Deutschland)"),
    ("it-IT", "Italian (Italy)", "Italiano (Italia)"),
    ("ja-JP", "Japanese (Japan)", "日本語 (日本)"),
    ("ko-KR", "Korean (Korea)", "한국어 (대한민국)"),
    ("pt-BR", "Portuguese (Brazil)", "Português (Brasil)"),
    ("ru-RU", "Russian (Russia)", "Русский (Россия)"),
    ("zh-CN", "Chinese (Mandarin, Simplified)", "中文 (简体)"),
    ("zh-TW", "Chinese (Taiwanese Mandarin, Traditional)", "中文 (繁體)"),
];

pub struct GoogleLanguagesComponent;

impl LanguagesGuest for GoogleLanguagesComponent {
    fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        Ok(LANGUAGES
            .iter()
            .map(|(code, name, native)| LanguageInfo {
                code: (*code).into(),
                name: (*name).into(),
                native_name: (*native).into(),
            })
            .collect())
    }
}
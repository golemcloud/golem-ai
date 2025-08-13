use golem_stt::golem::stt::languages::LanguageInfo;
use golem_stt::golem::stt::types::SttError;

pub struct DeepgramLanguagesComponent;

impl DeepgramLanguagesComponent {
    pub fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        let langs = vec![
            LanguageInfo { code: "en-US".into(), name: "English (United States)".into(), native_name: "English (United States)".into() },
            LanguageInfo { code: "en-GB".into(), name: "English (United Kingdom)".into(), native_name: "English (United Kingdom)".into() },
            LanguageInfo { code: "en-AU".into(), name: "English (Australia)".into(), native_name: "English (Australia)".into() },
            LanguageInfo { code: "en-CA".into(), name: "English (Canada)".into(), native_name: "English (Canada)".into() },
            LanguageInfo { code: "en-IN".into(), name: "English (India)".into(), native_name: "English (India)".into() },
            LanguageInfo { code: "es-ES".into(), name: "Spanish (Spain)".into(), native_name: "Español (España)".into() },
            LanguageInfo { code: "es-US".into(), name: "Spanish (United States)".into(), native_name: "Español (Estados Unidos)".into() },
            LanguageInfo { code: "es-MX".into(), name: "Spanish (Mexico)".into(), native_name: "Español (México)".into() },
            LanguageInfo { code: "fr-FR".into(), name: "French (France)".into(), native_name: "Français (France)".into() },
            LanguageInfo { code: "fr-CA".into(), name: "French (Canada)".into(), native_name: "Français (Canada)".into() },
            LanguageInfo { code: "de-DE".into(), name: "German (Germany)".into(), native_name: "Deutsch (Deutschland)".into() },
            LanguageInfo { code: "it-IT".into(), name: "Italian (Italy)".into(), native_name: "Italiano (Italia)".into() },
            LanguageInfo { code: "pt-BR".into(), name: "Portuguese (Brazil)".into(), native_name: "Português (Brasil)".into() },
            LanguageInfo { code: "pt-PT".into(), name: "Portuguese (Portugal)".into(), native_name: "Português (Portugal)".into() },
            LanguageInfo { code: "ja-JP".into(), name: "Japanese (Japan)".into(), native_name: "日本語 (日本)".into() },
            LanguageInfo { code: "ko-KR".into(), name: "Korean (Korea)".into(), native_name: "한국어 (대한민국)".into() },
            LanguageInfo { code: "zh-CN".into(), name: "Chinese (Mainland)".into(), native_name: "中文(中国大陆)".into() },
            LanguageInfo { code: "zh-TW".into(), name: "Chinese (Taiwan)".into(), native_name: "中文(台灣)".into() },
            LanguageInfo { code: "nl-NL".into(), name: "Dutch (Netherlands)".into(), native_name: "Nederlands (Nederland)".into() },
            LanguageInfo { code: "pl-PL".into(), name: "Polish (Poland)".into(), native_name: "Polski (Polska)".into() },
            LanguageInfo { code: "tr-TR".into(), name: "Turkish (Turkey)".into(), native_name: "Türkçe (Türkiye)".into() },
            LanguageInfo { code: "ru-RU".into(), name: "Russian (Russia)".into(), native_name: "Русский (Россия)".into() },
            LanguageInfo { code: "hi-IN".into(), name: "Hindi (India)".into(), native_name: "हिन्दी (भारत)".into() },
        ];
        Ok(langs)
    }
}

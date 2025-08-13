use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::types::SttError;

const LANGUAGES: &[(&str, &str, &str)] = &[
    ("en-US", "English (United States)", "English (United States)"),
    ("en-GB", "English (United Kingdom)", "English (United Kingdom)"),
    ("en-CA", "English (Canada)", "English (Canada)"),
    ("en-AU", "English (Australia)", "English (Australia)"),
    ("en-IN", "English (India)", "English (India)"),
    ("en-NZ", "English (New Zealand)", "English (New Zealand)"),
    ("es-ES", "Spanish (Spain)", "Español (España)"),
    ("es-MX", "Spanish (Mexico)", "Español (México)"),
    ("es-US", "Spanish (United States)", "Español (Estados Unidos)"),
    ("es-AR", "Spanish (Argentina)", "Español (Argentina)"),
    ("es-CL", "Spanish (Chile)", "Español (Chile)"),
    ("es-CO", "Spanish (Colombia)", "Español (Colombia)"),
    ("fr-FR", "French (France)", "Français (France)"),
    ("fr-CA", "French (Canada)", "Français (Canada)"),
    ("de-DE", "German (Germany)", "Deutsch (Deutschland)"),
    ("it-IT", "Italian (Italy)", "Italiano (Italia)"),
    ("nl-NL", "Dutch (Netherlands)", "Nederlands (Nederland)"),
    ("pl-PL", "Polish (Poland)", "Polski (Polska)"),
    ("tr-TR", "Turkish (Türkiye)", "Türkçe (Türkiye)"),
    ("pt-PT", "Portuguese (Portugal)", "Português (Portugal)"),
    ("pt-BR", "Portuguese (Brazil)", "Português (Brasil)"),
    ("hi-IN", "Hindi (India)", "हिन्दी (भारत)"),
    ("id-ID", "Indonesian (Indonesia)", "Bahasa Indonesia (Indonesia)"),
    ("th-TH", "Thai (Thailand)", "ไทย (ประเทศไทย)"),
    ("vi-VN", "Vietnamese (Vietnam)", "Tiếng Việt (Việt Nam)"),
    ("ja-JP", "Japanese (Japan)", "日本語 (日本)"),
    ("ko-KR", "Korean (Korea)", "한국어 (대한민국)"),
    ("yue-Hant-HK", "Chinese, Cantonese (Hong Kong)", "中文 粤语 (香港)"),
    ("zh-CN", "Chinese (Mandarin, Simplified)", "中文 (简体)"),
    ("zh-TW", "Chinese (Taiwanese Mandarin, Traditional)", "中文 (繁體)"),
    ("ar-SA", "Arabic (Saudi Arabia)", "العربية (السعودية)"),
    ("ar-EG", "Arabic (Egypt)", "العربية (مصر)"),
    ("ar-AE", "Arabic (United Arab Emirates)", "العربية (الإمارات)"),
    ("he-IL", "Hebrew (Israel)", "עברית (ישראל)"),
    ("ru-RU", "Russian (Russia)", "Русский (Россия)"),
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
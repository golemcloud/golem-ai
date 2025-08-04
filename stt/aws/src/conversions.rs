use golem_stt::golem::stt::languages::LanguageInfo;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_supported_languages() -> Vec<LanguageInfo> {
    vec![
        LanguageInfo {
            code: "en-US".to_string(),
            name: "English (United States)".to_string(),
            native_name: "English (United States)".to_string(),
        },
        LanguageInfo {
            code: "en-GB".to_string(),
            name: "English (United Kingdom)".to_string(),
            native_name: "English (United Kingdom)".to_string(),
        },
        LanguageInfo {
            code: "es-ES".to_string(),
            name: "Spanish (Spain)".to_string(),
            native_name: "Español (España)".to_string(),
        },
        LanguageInfo {
            code: "es-US".to_string(),
            name: "Spanish (United States)".to_string(),
            native_name: "Español (Estados Unidos)".to_string(),
        },
        LanguageInfo {
            code: "fr-FR".to_string(),
            name: "French (France)".to_string(),
            native_name: "Français (France)".to_string(),
        },
        LanguageInfo {
            code: "de-DE".to_string(),
            name: "German (Germany)".to_string(),
            native_name: "Deutsch (Deutschland)".to_string(),
        },
        LanguageInfo {
            code: "it-IT".to_string(),
            name: "Italian (Italy)".to_string(),
            native_name: "Italiano (Italia)".to_string(),
        },
        LanguageInfo {
            code: "pt-BR".to_string(),
            name: "Portuguese (Brazil)".to_string(),
            native_name: "Português (Brasil)".to_string(),
        },
        LanguageInfo {
            code: "ru-RU".to_string(),
            name: "Russian (Russia)".to_string(),
            native_name: "Русский (Россия)".to_string(),
        },
        LanguageInfo {
            code: "ja-JP".to_string(),
            name: "Japanese (Japan)".to_string(),
            native_name: "日本語（日本）".to_string(),
        },
        LanguageInfo {
            code: "zh-CN".to_string(),
            name: "Chinese (Simplified, China)".to_string(),
            native_name: "中文（简体，中国）".to_string(),
        },
        LanguageInfo {
            code: "ko-KR".to_string(),
            name: "Korean (South Korea)".to_string(),
            native_name: "한국어 (대한민국)".to_string(),
        },
        LanguageInfo {
            code: "ar-SA".to_string(),
            name: "Arabic (Saudi Arabia)".to_string(),
            native_name: "العربية (المملكة العربية السعودية)".to_string(),
        },
        LanguageInfo {
            code: "hi-IN".to_string(),
            name: "Hindi (India)".to_string(),
            native_name: "हिन्दी (भारत)".to_string(),
        },
        LanguageInfo {
            code: "nl-NL".to_string(),
            name: "Dutch (Netherlands)".to_string(),
            native_name: "Nederlands (Nederland)".to_string(),
        },
    ]
}

// Helper function to generate unique job names
pub fn generate_job_name() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("golem-stt-job-{}", timestamp)
}
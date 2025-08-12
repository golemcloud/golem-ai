use golem_stt::golem::stt::languages::{Guest as LanguagesGuest, LanguageInfo};
use golem_stt::golem::stt::types::SttError;

const LANGUAGES: &[(&str, &str, &str)] = &[
    ("af-ZA", "Afrikaans (South Africa)", "Afrikaans (South Africa)"),
    ("am-ET", "Amharic (Ethiopia)", "Amharic (Ethiopia)"),
    ("ar-SA", "Arabic (Saudi Arabia)", "العربية (السعودية)"),
    ("ar-EG", "Arabic (Egypt)", "العربية (مصر)"),
    ("bg-BG", "Bulgarian (Bulgaria)", "Български (България)"),
    ("bn-IN", "Bengali (India)", "বাংলা (ভারত)"),
    ("ca-ES", "Catalan", "Català"),
    ("cs-CZ", "Czech (Czechia)", "Čeština (Česko)"),
    ("da-DK", "Danish (Denmark)", "Dansk (Danmark)"),
    ("de-DE", "German (Germany)", "Deutsch (Deutschland)"),
    ("de-AT", "German (Austria)", "Deutsch (Österreich)"),
    ("de-CH", "German (Switzerland)", "Deutsch (Schweiz)"),
    ("el-GR", "Greek (Greece)", "Ελληνικά (Ελλάδα)"),
    ("en-US", "English (United States)", "English (United States)"),
    ("en-GB", "English (United Kingdom)", "English (United Kingdom)"),
    ("en-AU", "English (Australia)", "English (Australia)"),
    ("en-CA", "English (Canada)", "English (Canada)"),
    ("en-IN", "English (India)", "English (India)"),
    ("en-NZ", "English (New Zealand)", "English (New Zealand)"),
    ("en-ZA", "English (South Africa)", "English (South Africa)"),
    ("es-ES", "Spanish (Spain)", "Español (España)"),
    ("es-MX", "Spanish (Mexico)", "Español (México)"),
    ("es-US", "Spanish (United States)", "Español (Estados Unidos)"),
    ("fi-FI", "Finnish (Finland)", "Suomi (Suomi)"),
    ("fr-FR", "French (France)", "Français (France)"),
    ("fr-CA", "French (Canada)", "Français (Canada)"),
    ("he-IL", "Hebrew (Israel)", "עברית (ישראל)"),
    ("hi-IN", "Hindi (India)", "हिन्दी (भारत)"),
    ("hr-HR", "Croatian (Croatia)", "Hrvatski (Hrvatska)"),
    ("hu-HU", "Hungarian (Hungary)", "Magyar (Magyarország)"),
    ("id-ID", "Indonesian (Indonesia)", "Bahasa Indonesia (Indonesia)"),
    ("it-IT", "Italian (Italy)", "Italiano (Italia)"),
    ("ja-JP", "Japanese (Japan)", "日本語 (日本)"),
    ("ko-KR", "Korean (Korea)", "한국어 (대한민국)"),
    ("lt-LT", "Lithuanian (Lithuania)", "Lietuvių (Lietuva)"),
    ("lv-LV", "Latvian (Latvia)", "Latviešu (Latvija)"),
    ("nb-NO", "Norwegian Bokmål (Norway)", "Norsk bokmål (Norge)"),
    ("nl-NL", "Dutch (Netherlands)", "Nederlands (Nederland)"),
    ("pl-PL", "Polish (Poland)", "Polski (Polska)"),
    ("pt-BR", "Portuguese (Brazil)", "Português (Brasil)"),
    ("pt-PT", "Portuguese (Portugal)", "Português (Portugal)"),
    ("ro-RO", "Romanian (Romania)", "Română (România)"),
    ("ru-RU", "Russian (Russia)", "Русский (Россия)"),
    ("sk-SK", "Slovak (Slovakia)", "Slovenčina (Slovensko)"),
    ("sl-SI", "Slovenian (Slovenia)", "Slovenščina (Slovenija)"),
    ("sv-SE", "Swedish (Sweden)", "Svenska (Sverige)"),
    ("ta-IN", "Tamil (India)", "தமிழ் (இந்தியா)"),
    ("te-IN", "Telugu (India)", "తెలుగు (భారతదేశం)"),
    ("th-TH", "Thai (Thailand)", "ไทย (ประเทศไทย)"),
    ("tr-TR", "Turkish (Türkiye)", "Türkçe (Türkiye)"),
    ("uk-UA", "Ukrainian (Ukraine)", "Українська (Україна)"),
    ("vi-VN", "Vietnamese (Vietnam)", "Tiếng Việt (Việt Nam)"),
    ("yue-CN", "Chinese (Cantonese, Simplified)", "粤语 (简体)"),
    ("zh-CN", "Chinese (Mandarin, Simplified)", "中文 (简体)"),
    ("zh-TW", "Chinese (Taiwanese Mandarin, Traditional)", "中文 (繁體)"),
    ("zh-HK", "Chinese (Cantonese, Traditional)", "中文 (香港)"),
];

pub struct AzureLanguagesComponent;

impl LanguagesGuest for AzureLanguagesComponent {
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
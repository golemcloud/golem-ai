use golem_stt::golem::stt::languages::{LanguageInfo};
use golem_stt::golem::stt::types::SttError;

pub struct WhisperLanguagesComponent;

impl WhisperLanguagesComponent {
    pub fn list_languages() -> Result<Vec<LanguageInfo>, SttError> {
        let langs = vec![
            LanguageInfo { code: "en".into(), name: "English".into(), native_name: "English".into() },
            LanguageInfo { code: "es".into(), name: "Spanish".into(), native_name: "Español".into() },
            LanguageInfo { code: "fr".into(), name: "French".into(), native_name: "Français".into() },
            LanguageInfo { code: "de".into(), name: "German".into(), native_name: "Deutsch".into() },
            LanguageInfo { code: "it".into(), name: "Italian".into(), native_name: "Italiano".into() },
            LanguageInfo { code: "pt".into(), name: "Portuguese".into(), native_name: "Português".into() },
            LanguageInfo { code: "ja".into(), name: "Japanese".into(), native_name: "日本語".into() },
            LanguageInfo { code: "ko".into(), name: "Korean".into(), native_name: "한국어".into() },
            LanguageInfo { code: "zh".into(), name: "Chinese".into(), native_name: "中文".into() },
            LanguageInfo { code: "ar".into(), name: "Arabic".into(), native_name: "العربية".into() },
            LanguageInfo { code: "ru".into(), name: "Russian".into(), native_name: "Русский".into() },
            LanguageInfo { code: "tr".into(), name: "Turkish".into(), native_name: "Türkçe".into() },
            LanguageInfo { code: "pl".into(), name: "Polish".into(), native_name: "Polski".into() },
            LanguageInfo { code: "nl".into(), name: "Dutch".into(), native_name: "Nederlands".into() },
            LanguageInfo { code: "sv".into(), name: "Swedish".into(), native_name: "Svenska".into() },
            LanguageInfo { code: "da".into(), name: "Danish".into(), native_name: "Dansk".into() },
            LanguageInfo { code: "fi".into(), name: "Finnish".into(), native_name: "Suomi".into() },
            LanguageInfo { code: "he".into(), name: "Hebrew".into(), native_name: "עברית".into() },
            LanguageInfo { code: "uk".into(), name: "Ukrainian".into(), native_name: "Українська".into() },
            LanguageInfo { code: "cs".into(), name: "Czech".into(), native_name: "Čeština".into() },
            LanguageInfo { code: "ro".into(), name: "Romanian".into(), native_name: "Română".into() },
            LanguageInfo { code: "hu".into(), name: "Hungarian".into(), native_name: "Magyar".into() },
            LanguageInfo { code: "el".into(), name: "Greek".into(), native_name: "Ελληνικά".into() },
            LanguageInfo { code: "th".into(), name: "Thai".into(), native_name: "ไทย".into() },
            LanguageInfo { code: "vi".into(), name: "Vietnamese".into(), native_name: "Tiếng Việt".into() },
            LanguageInfo { code: "id".into(), name: "Indonesian".into(), native_name: "Bahasa Indonesia".into() },
            LanguageInfo { code: "hi".into(), name: "Hindi".into(), native_name: "हिन्दी".into() },
            LanguageInfo { code: "bn".into(), name: "Bengali".into(), native_name: "বাংলা".into() },
            LanguageInfo { code: "ta".into(), name: "Tamil".into(), native_name: "தமிழ்".into() },
            LanguageInfo { code: "te".into(), name: "Telugu".into(), native_name: "తెలుగు".into() },
            LanguageInfo { code: "ur".into(), name: "Urdu".into(), native_name: "اردو".into() },
            LanguageInfo { code: "fa".into(), name: "Persian".into(), native_name: "فارسی".into() },
        ];
        Ok(langs)
    }
}

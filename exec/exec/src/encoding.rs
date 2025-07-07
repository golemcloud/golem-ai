use crate::error::validation;
use crate::types::{Encoding, Error, File};
use base64::{engine::general_purpose, Engine as _};

pub fn decode_content(content: &str, encoding: Encoding) -> Result<Vec<u8>, Error> {
    match encoding {
        Encoding::Utf8 => Ok(content.as_bytes().to_vec()),
        Encoding::Base64 => general_purpose::STANDARD
            .decode(content)
            .map_err(|e| validation::base64_decode_failed(&e.to_string())),
        Encoding::Hex => {
            decode_hex(content).map_err(|e| validation::hex_decode_failed(&e.to_string()))
        }
    }
}

pub fn encode_content(content: &[u8], encoding: Encoding) -> Result<String, Error> {
    match encoding {
        Encoding::Utf8 => match std::str::from_utf8(content) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(validation::invalid_encoding(
                "content",
                "UTF-8",
                &e.to_string(),
            )),
        },
        Encoding::Base64 => {
            let encoded = general_purpose::STANDARD.encode(content);
            Ok(encoded)
        }
        Encoding::Hex => {
            let encoded = encode_hex(content);
            Ok(encoded)
        }
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("Hex string must have even length".to_string());
    }

    let mut result = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let hex_str =
            std::str::from_utf8(chunk).map_err(|e| format!("Invalid UTF-8 in hex string: {e}"))?;

        let byte =
            u8::from_str_radix(hex_str, 16).map_err(|e| format!("Invalid hex digit: {e}"))?;

        result.push(byte);
    }

    Ok(result)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn detect_encoding(content: &[u8]) -> Encoding {
    // Try UTF-8 first
    if std::str::from_utf8(content).is_ok() {
        // Check if it looks like base64
        if let Ok(content_str) = std::str::from_utf8(content) {
            let trimmed = content_str.trim();
            if is_likely_base64(trimmed) {
                return Encoding::Base64;
            }
            if is_likely_hex(trimmed) {
                return Encoding::Hex;
            }
        }
        Encoding::Utf8
    } else {
        // Binary content, default to base64 for transport
        Encoding::Base64
    }
}

fn is_likely_base64(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Base64 should only contain valid characters
    let valid_chars = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');

    if !valid_chars {
        return false;
    }

    // Should be multiple of 4 in length (with padding)
    let without_padding = s.trim_end_matches('=');
    let padding_count = s.len() - without_padding.len();

    padding_count <= 2 && (s.len() % 4 == 0)
}

fn is_likely_hex(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Should be even length
    if s.len() % 2 != 0 {
        return false;
    }

    // Should only contain hex digits
    s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn validate_file_encoding(file: &File) -> Result<(), Error> {
    if let Some(ref encoding) = file.encoding {
        match encoding {
            Encoding::Utf8 => {
                if let Err(e) = std::str::from_utf8(&file.content) {
                    return Err(validation::decode_error(&file.name, "UTF-8", e));
                }
            }
            Encoding::Base64 => {
                let content_str = std::str::from_utf8(&file.content)
                    .map_err(|e| validation::decode_error(&file.name, "Base64", e))?;
                if let Err(e) = general_purpose::STANDARD.decode(content_str.trim()) {
                    return Err(validation::decode_error(&file.name, "Base64", e));
                }
            }
            Encoding::Hex => {
                let content_str = std::str::from_utf8(&file.content)
                    .map_err(|e| validation::decode_error(&file.name, "Hex", e))?;
                if let Err(e) = decode_hex(content_str.trim()) {
                    return Err(validation::decode_error(&file.name, "Hex", e));
                }
            }
        }
    }
    Ok(())
}

pub fn decode_file_content(file: &File) -> Result<Vec<u8>, Error> {
    match file.encoding.unwrap_or(Encoding::Utf8) {
        Encoding::Utf8 => Ok(file.content.to_vec()),
        Encoding::Base64 => {
            let content_str = std::str::from_utf8(&file.content)
                .map_err(|e| validation::decode_error(&file.name, "Base64", e))?;
            general_purpose::STANDARD
                .decode(content_str.trim())
                .map_err(|e| validation::decode_error(&file.name, "Base64", e))
        }
        Encoding::Hex => {
            let content_str = std::str::from_utf8(&file.content)
                .map_err(|e| validation::decode_error(&file.name, "Hex", e))?;
            decode_hex(content_str.trim())
                .map_err(|e| validation::decode_error(&file.name, "Hex", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_encoding() {
        let content = "Hello, World!";
        let decoded = decode_content(content, Encoding::Utf8).unwrap();
        assert_eq!(decoded, content.as_bytes());

        let encoded = encode_content(&decoded, Encoding::Utf8).unwrap();
        assert_eq!(encoded, content);
    }

    #[test]
    fn test_base64_encoding() {
        let content = "Hello, World!";
        let base64_content = general_purpose::STANDARD.encode(content);

        let decoded = decode_content(&base64_content, Encoding::Base64).unwrap();
        assert_eq!(decoded, content.as_bytes());

        let encoded = encode_content(content.as_bytes(), Encoding::Base64).unwrap();
        assert_eq!(encoded, base64_content);
    }

    #[test]
    fn test_hex_encoding() {
        let content = "Hello";
        let hex_content = "48656c6c6f";

        let decoded = decode_content(hex_content, Encoding::Hex).unwrap();
        assert_eq!(decoded, content.as_bytes());

        let encoded = encode_content(content.as_bytes(), Encoding::Hex).unwrap();
        assert_eq!(encoded, hex_content);
    }

    #[test]
    fn test_detect_encoding() {
        assert_eq!(detect_encoding("Hello".as_bytes()), Encoding::Utf8);
        assert_eq!(detect_encoding("SGVsbG8=".as_bytes()), Encoding::Base64);
        assert_eq!(detect_encoding("48656c6c6f".as_bytes()), Encoding::Hex);
    }
}

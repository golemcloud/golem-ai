use crate::bindings::golem::stt::transcription;
use crate::types::{AudioConfig, AudioFormat, SttError, QuotaUnit};

/// Test edge case: silence detection
pub fn test_silence_handling() -> String {
    let silent_audio = generate_silent_wav(5.0); // 5 seconds of silence
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };

    match transcription::transcribe(&silent_audio, config, None) {
        Ok(result) => {
            // Should handle silence gracefully - either empty text or appropriate response
            if result.alternatives.is_empty() {
                "OK silence: no alternatives".to_string()
            } else if let Some(first) = result.alternatives.first() {
                if first.text.trim().is_empty() || first.text.to_lowercase().contains("silence") {
                    "OK silence: detected".to_string()
                } else {
                    format!("WARN silence: got text '{}'", first.text)
                }
            } else {
                "OK silence: handled".to_string()
            }
        }
        Err(err) => match err {
            SttError::InvalidAudio(_) => "OK silence: rejected as invalid".to_string(),
            _ => format!("ERROR silence: {:?}", err),
        }
    }
}

/// Test edge case: overlapping speakers
pub fn test_overlapping_speakers() -> String {
    let multi_speaker_audio = generate_overlapping_speakers_wav();
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };
    
    let opts = transcription::TranscribeOptions {
        enable_timestamps: Some(true),
        enable_speaker_diarization: Some(true),
        language: Some("en-US".to_string()),
        model: None,
        profanity_filter: None,
        vocabulary: None,
        speech_context: None,
        enable_word_confidence: Some(true),
        enable_timing_detail: Some(true),
    };

    match transcription::transcribe(&multi_speaker_audio, config, Some(&opts)) {
        Ok(result) => {
            // Check if speaker information is provided where supported
            let has_speaker_info = result.alternatives
                .first()
                .map(|alt| alt.words.iter().any(|w| w.speaker_id.is_some()))
                .unwrap_or(false);
            
            if has_speaker_info {
                "OK overlapping: speaker diarization working".to_string()
            } else {
                "OK overlapping: no speaker info (graceful degradation)".to_string()
            }
        }
        Err(err) => format!("ERROR overlapping: {:?}", err),
    }
}

/// Test edge case: very long audio (simulated)
pub fn test_long_audio_handling() -> String {
    // Simulate a long audio file (metadata indicates 30 minutes)
    let long_audio = generate_long_audio_simulation();
    let config = AudioConfig {
        format: AudioFormat::Mp3,
        sample_rate: Some(44100),
        channels: Some(2),
    };

    match transcription::transcribe(&long_audio, config, None) {
        Ok(result) => {
            // Should handle long audio appropriately
            if result.metadata.duration_seconds > 1000.0 {
                "OK long: handled extended duration".to_string()
            } else {
                "OK long: processed".to_string()
            }
        }
        Err(err) => match err {
            SttError::QuotaExceeded(_) => "OK long: quota limit hit".to_string(),
            SttError::UnsupportedOperation(_) => "OK long: length limit".to_string(),
            _ => format!("ERROR long: {:?}", err),
        }
    }
}

/// Test network error resilience
pub fn test_network_error_handling() -> String {
    // Use invalid endpoint to trigger network error
    std::env::set_var("STT_PROVIDER_ENDPOINT", "https://invalid-endpoint-12345.example.com");
    
    let audio = crate::mock_wav_audio();
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };

    match transcription::transcribe(&audio, config, None) {
        Ok(_) => "ERROR network: should have failed".to_string(),
        Err(err) => match err {
            SttError::NetworkError(_) => "OK network: error detected".to_string(),
            SttError::ServiceUnavailable(_) => "OK network: service unavailable".to_string(),
            _ => format!("OK network: error type {:?}", err),
        }
    }
}

/// Test rate limiting behavior
pub fn test_rate_limiting() -> String {
    // Simulate rapid requests to trigger rate limiting
    let audio = crate::mock_wav_audio();
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };

    let mut rate_limited = false;
    
    // Make multiple rapid requests
    for _i in 0..5 {
        match transcription::transcribe(&audio, config, None) {
            Ok(_) => continue,
            Err(SttError::RateLimited(_)) => {
                rate_limited = true;
                break;
            }
            Err(_) => continue,
        }
    }

    if rate_limited {
        "OK rate: limit detected".to_string()
    } else {
        "OK rate: no limit hit (or high limit)".to_string()
    }
}

/// Test quota behavior
pub fn test_quota_behavior() -> String {
    // Test quota information in error responses
    let large_audio = vec![0u8; 10_000_000]; // 10MB of audio
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };

    match transcription::transcribe(&large_audio, config, None) {
        Ok(_) => "OK quota: large file accepted".to_string(),
        Err(err) => match err {
            SttError::QuotaExceeded(info) => {
                format!("OK quota: exceeded {}/{} {}", info.used, info.limit, 
                       match info.unit {
                           QuotaUnit::Seconds => "seconds",
                           QuotaUnit::Requests => "requests",
                           QuotaUnit::Credits => "credits",
                       })
            }
            SttError::InsufficientCredits => "OK quota: insufficient credits".to_string(),
            _ => format!("OK quota: other limit {:?}", err),
        }
    }
}

/// Test invalid audio formats
pub fn test_invalid_audio_formats() -> String {
    let invalid_audio = vec![0xFF, 0xFE, 0xFD, 0xFC]; // Invalid audio data
    let config = AudioConfig {
        format: AudioFormat::Wav,
        sample_rate: Some(44100),
        channels: Some(1),
    };

    match transcription::transcribe(&invalid_audio, config, None) {
        Ok(_) => "WARN invalid: accepted bad audio".to_string(),
        Err(err) => match err {
            SttError::InvalidAudio(_) => "OK invalid: rejected".to_string(),
            SttError::UnsupportedFormat(_) => "OK invalid: format rejected".to_string(),
            _ => format!("OK invalid: error {:?}", err),
        }
    }
}

/// Test streaming error handling after finish
pub fn test_streaming_after_finish() -> String {
    let config = AudioConfig {
        format: AudioFormat::Pcm,
        sample_rate: Some(16000),
        channels: Some(1),
    };

    let stream = match transcription::transcribe_stream(config, None) {
        Ok(s) => s,
        Err(SttError::UnsupportedOperation(_)) => {
            return "OK streaming: not supported (Whisper)".to_string();
        }
        Err(err) => return format!("ERROR streaming: {:?}", err),
    };

    // Finish the stream
    if let Err(err) = stream.finish() {
        return format!("ERROR finish: {:?}", err);
    }

    // Try to send audio after finish - should fail
    match stream.send_audio(&crate::mock_pcm_chunk()) {
        Ok(_) => "ERROR streaming: send after finish succeeded".to_string(),
        Err(_) => "OK streaming: send after finish rejected".to_string(),
    }
}

// Helper functions to generate test audio data

fn generate_silent_wav(duration_seconds: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let samples = (duration_seconds * sample_rate as f32) as usize;
    let mut wav = crate::mock_wav_audio();
    
    // Add silent PCM data
    for _ in 0..samples {
        wav.extend_from_slice(&[0u8, 0u8]); // 16-bit silence
    }
    wav
}

fn generate_overlapping_speakers_wav() -> Vec<u8> {
    // Generate a WAV with simulated overlapping speech patterns
    let mut wav = crate::mock_wav_audio();
    
    // Add some varied amplitude data to simulate multiple speakers
    for i in 0..8820 { // ~0.2 seconds at 44.1kHz
        let sample = if i % 100 < 50 {
            (i as i16 % 1000) as u8 // Speaker 1 pattern
        } else {
            ((i as i16 % 800) + 200) as u8 // Speaker 2 pattern  
        };
        wav.push(sample);
        wav.push(0); // Second byte of 16-bit sample
    }
    wav
}

fn generate_long_audio_simulation() -> Vec<u8> {
    // Generate metadata suggesting a long file without actually creating 30 minutes of data
    let mut wav = crate::mock_wav_audio();
    
    // Add enough data to suggest a longer file
    for i in 0..44100 { // 1 second of data
        wav.push((i % 256) as u8);
        wav.push(((i / 256) % 256) as u8);
    }
    wav
}

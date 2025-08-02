# Whisper STT Component

## Graceful Degradation
- Speaker diarization and word confidence are not natively supported - these fields will be returned as `none`
- Streaming transcription is not available - `transcribe-stream` will return an `UnsupportedOperation` error
- Custom vocabularies and speech context are stored in durable storage but not used during transcription
- Word-level timestamps are supported via WhisperX integration (set `WHISPERX_ENDPOINT` environment variable)

Implements the `golem:stt` WIT interface using OpenAI Whisper (or compatible gateway) as a WASI 0.23 component.

Features
- Batch transcription only
- Timestamps supported when provided by the gateway/tooling
- No speaker diarization or word confidence (graceful degradation)

Environment
Common:
- STT_PROVIDER_ENDPOINT: Base endpoint used by the client
- STT_PROVIDER_TIMEOUT: Request timeout seconds (default 30)
- STT_PROVIDER_MAX_RETRIES: Max retries (default 3)
- STT_PROVIDER_LOG_LEVEL: trace|debug|info|warn|error

Provider-Specific:
- OPENAI_API_KEY: OpenAI API key for Whisper access
- WHISPERX_ENDPOINT: Optional endpoint for WhisperX word-level timestamps

Degradation
- Speaker diarization and word confidence are not provided; corresponding fields are `None`
- Streaming is not implemented for Whisper

Build
- `cargo component build -p stt-whisper`

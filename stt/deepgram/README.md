# Deepgram STT Component

## Graceful Degradation
- All features supported - no degradation needed

Implements the `golem:stt` WIT interface for Deepgram using a WASI 0.23 component.

Features
- Batch transcription with timestamps and alternatives
- Language and model selection where supported
- Graceful degradation when fields are unavailable

Environment
Common:
- STT_PROVIDER_ENDPOINT: Base endpoint used by the client; for streaming, `/stream` is appended
- STT_PROVIDER_TIMEOUT: Request timeout seconds (default 30)
- STT_PROVIDER_MAX_RETRIES: Max retries (default 3)
- STT_PROVIDER_LOG_LEVEL: trace|debug|info|warn|error

Provider-Specific:
- DEEPGRAM_API_KEY: API key for Deepgram authentication

Streaming
Native WebSockets are not available in WASI components. This component does not implement streaming and returns `unsupported-operation` for `transcribe-stream`.

Degradation
- If diarization or word confidence are not available, they are omitted (None).
- Errors are mapped to `types::stt-error` with specific variants where possible.

Build
- `cargo component build -p stt-deepgram`

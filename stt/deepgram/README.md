# Deepgram STT Component

## Graceful Degradation
- All features supported - no degradation needed

Implements the `golem:stt` WIT interface for Deepgram using a WASI 0.23 component.

Features
- Batch transcription with timestamps and alternatives
- Emulated streaming via HTTP gateway (see below)
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

Streaming (Emulated)
Because native WebSockets are limited in WASI, streaming is emulated via a gateway that exposes:

- POST {STT_PROVIDER_ENDPOINT}/stream/send  body: { request_id, chunk_b64 }
- POST {STT_PROVIDER_ENDPOINT}/stream/finish body: { request_id }
- GET  {STT_PROVIDER_ENDPOINT}/stream/recv?request_id=... -> { alternative } or 204

The component constructs content-type from the AudioConfig.

Degradation
- If diarization or word confidence are not available, they are omitted (None).
- Errors are mapped to `types::stt-error` with specific variants where possible.

Build
- `cargo component build -p stt-deepgram`

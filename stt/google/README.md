# Google STT Component

## Graceful Degradation
- Streaming is not supported in WASI; `transcribe-stream` returns `unsupported-operation`

Implements the `golem:stt` WIT interface for Google Cloud Speech-to-Text using a WASI 0.23 component.

Features
- Batch transcription with timestamps and alternatives
- Optional language and model selection
- Graceful degradation when fields are unavailable

Environment
Common:
- STT_PROVIDER_ENDPOINT: Base endpoint used by the client; for streaming, `/stream` is appended
- STT_PROVIDER_TIMEOUT: Request timeout seconds (default 30)
- STT_PROVIDER_MAX_RETRIES: Max retries (default 3)
- STT_PROVIDER_LOG_LEVEL: trace|debug|info|warn|error

Provider-Specific:
- GOOGLE_APPLICATION_CREDENTIALS: Path to service account JSON file
- GOOGLE_CLOUD_PROJECT: Google Cloud project ID
- GOOGLE_ACCESS_TOKEN: Optional; if not provided, the component derives an access token from the service account JSON

Streaming
Native WebSockets/gRPC for Google STT are not available in WASI components. This component does not implement streaming and returns `unsupported-operation` for `transcribe-stream`.

Degradation
- If diarization or word confidence are not available, they are omitted (None).
- Errors are mapped to `types::stt-error` with specific variants where possible.

Build
- `cargo component build -p stt-google`

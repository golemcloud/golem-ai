# AWS STT Component

## Graceful Degradation
- Content redaction requires specific configuration
- Custom vocabularies require AWS console setup

Implements the `golem:stt` WIT interface for Amazon Transcribe using a WASI 0.23 component.

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
- AWS_ACCESS_KEY_ID: AWS access key ID
- AWS_SECRET_ACCESS_KEY: AWS secret access key
- AWS_SESSION_TOKEN: AWS session token (optional, for temporary credentials)
- AWS_REGION: AWS region (e.g., us-east-1, us-west-2)

Streaming
Native WebSockets/gRPC are not available in WASI components. This component does not implement streaming and returns `unsupported-operation` for `transcribe-stream`.

Degradation
- If diarization or word confidence are not available, they are omitted (None).
- Errors are mapped to `types::stt-error` with specific variants where possible.

Build
- `cargo component build -p stt-aws`

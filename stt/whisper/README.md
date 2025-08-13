Whisper STT Component

Implements the golem:stt interface for OpenAI Whisper (WASI 0.23).

Env vars:
- OPENAI_API_KEY
- STT_PROVIDER_ENDPOINT (default: https://api.openai.com/v1)
- STT_PROVIDER_TIMEOUT (default: 30)
- STT_PROVIDER_MAX_RETRIES (default: 3)
- STT_MAX_CONCURRENCY (native only)
- WHISPER_MODEL (default: whisper-1)

Behavior:
- No diarization or word confidence: fields are None.
- speech-context merged to prompt.

Build:
- cargo component build -p golem-stt-whisper

# Azure Speech-to-Text Component

Azure Speech-to-Text for WebAssembly and Golem Cloud.

## Environment Variables

- AZURE_SPEECH_KEY
- AZURE_SPEECH_REGION
- STT_PROVIDER_ENDPOINT
- STT_PROVIDER_TIMEOUT
- STT_PROVIDER_MAX_RETRIES
- STT_BUFFER_LIMIT_BYTES
- STT_PROVIDER_LOG_LEVEL

## Build

```bash
cargo component build -p golem-stt-azure
```

## Tests

```bash
cargo test -p golem-stt-azure
```




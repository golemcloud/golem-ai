# Azure Speech-to-Text Component

Azure Speech-to-Text for WebAssembly and Golem Cloud.

## Key Notes

- Streaming API has been removed. Use multi-transcribe to run multiple batch jobs concurrently and return all results together.
- Languages list is a snapshot; see `stt/azure/src/languages.rs`.

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




# Google Speech-to-Text Component

This component wraps Google Cloud Speech-to-Text so it can be consumed from a WebAssembly sandbox (and Golem Cloud runtime).

## Supported Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `GOOGLE_APPLICATION_CREDENTIALS` (required) | – |  Either the *contents* of the service-account JSON or a filesystem path to it. |
| `STT_PROVIDER_ENDPOINT` | Google production endpoint | Override for testing or proxying. |
| `STT_PROVIDER_TIMEOUT` | `30` | HTTP timeout in seconds. |
| `STT_PROVIDER_MAX_RETRIES` | `3` | Automatic retry limit on 5xx / network errors. |
| `STT_PROVIDER_LOG_LEVEL` | `info` | `trace`, `debug`, `info`, etc. |
| `GOOGLE_CLOUD_PROJECT` | – | Project id to bill against (optional; taken from creds if omitted). |

## Batch Transcription (implemented)

`transcribe(audio, config, options)` loads the above variables, exchanges the service-account JWT for an OAuth token and invokes the Speech API’s *Recognize* method.

## Streaming Transcription (future work)

`transcribe_stream` is stubbed and currently returns `UnsupportedOperation`. When enabled, the component will open a bidirectional gRPC stream to `/v1/speech:streamingRecognize`.

### Graceful degradation

• If streaming is requested, callers should fall back to batch mode on `UnsupportedOperation`.
• Network / quota / auth failures are mapped to `SttError` variants – see `error::map_http_status`.

## Build

```bash
cargo component build -p golem-stt-google
```

The Makefile already covers all domains:

```bash
cargo make build stt   # regular
cargo make build-portable stt  # portable variant
```

## Tests

```bash
cargo test -p golem-stt-google
``` 
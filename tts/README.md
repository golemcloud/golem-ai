# golem-tts

WebAssembly Components providing a unified API for various Text-to-Speech (TTS) providers.

## Versions

Each TTS provider has two versions: **Default** (with Golem-specific durability features) and **Portable** (no Golem dependencies).

| Name                         | Description                                                                                |
|------------------------------|--------------------------------------------------------------------------------------------|
| `golem-tts-elevenlabs.wasm`  | TTS implementation for ElevenLabs, with Golem durability features                          |
| `golem-tts-polly.wasm`       | TTS implementation for AWS Polly, with Golem durability features                           |
| `golem-tts-google.wasm`      | TTS implementation for Google Cloud Text-to-Speech, with Golem durability features         |
| `golem-tts-deepgram.wasm`    | TTS implementation for Deepgram Aura, with Golem durability features                        |
| `golem-tts-elevenlabs-portable.wasm` | Portable ElevenLabs implementation                                                  |
| `golem-tts-polly-portable.wasm`      | Portable AWS Polly implementation                                                   |
| `golem-tts-google-portable.wasm`     | Portable Google Cloud Text-to-Speech implementation                                 |
| `golem-tts-deepgram-portable.wasm`   | Portable Deepgram Aura implementation                                              |

Every component exports the same `golem:tts` interface, [defined here](tts/wit/golem-tts.wit).

## Environment Variables

The TTS components read configuration from environment variables. The table below summarizes the
shared settings and provider-specific requirements.

### Common configuration

| Variable | Required | Description |
| --- | --- | --- |
| `TTS_PROVIDER_TIMEOUT` | No | Request timeout in seconds (default: `30`). |
| `TTS_PROVIDER_MAX_RETRIES` | No | Maximum retry attempts for transient failures (default: `3`). |
| `TTS_PROVIDER_LOG_LEVEL` | No | Logging verbosity (`trace`, `debug`, `info`, `warn`, `error`). |

### ElevenLabs

| Variable | Required | Description |
| --- | --- | --- |
| `ELEVENLABS_API_KEY` | Yes | API key for ElevenLabs. |
| `ELEVENLABS_MODEL_VERSION` | No | Optional model version (falls back to provider default). |

### AWS Polly

| Variable | Required | Description |
| --- | --- | --- |
| `AWS_REGION` | Yes | AWS region (for example `us-east-1`). |
| `AWS_ACCESS_KEY_ID` | Yes | AWS access key ID. |
| `AWS_SECRET_ACCESS_KEY` | Yes | AWS secret access key. |
| `AWS_SESSION_TOKEN` | No | AWS session token (required for temporary credentials). |

### Google Cloud Text-to-Speech

| Variable | Required | Description |
| --- | --- | --- |
| `GOOGLE_APPLICATION_CREDENTIALS` | Yes* | Path to a service account JSON file. |
| `GOOGLE_CLOUD_PROJECT` | Yes* | Project ID when using inline credentials. |
| `GOOGLE_CLIENT_EMAIL` | Yes* | Client email when using inline credentials. |
| `GOOGLE_PRIVATE_KEY` | Yes* | Private key when using inline credentials. |

`*` Either `GOOGLE_APPLICATION_CREDENTIALS` **or** the inline credential trio is required.

### Deepgram Aura

| Variable | Required | Description |
| --- | --- | --- |
| `DEEPGRAM_API_KEY` | Yes | Deepgram API key. |
| `DEEPGRAM_API_VERSION` | No | Optional API version override. |

## Example Usage

The [test application](../test/tts/components-rust/test-tts/src/lib.rs) demonstrates the exported
functions and provider presets. Use it to validate your credentials or build pipeline.

### Running the test application

1. Start a Golem instance.
2. Build and deploy the test component for the desired provider:

```bash
cd ../test/tts
golem build --preset elevenlabs-debug
golem deploy --preset elevenlabs-debug
```

3. Start a worker with the required provider environment variables:

```bash
golem worker new test:tts/debug \
  --env ELEVENLABS_API_KEY=your_key_here \
  --env TTS_PROVIDER_LOG_LEVEL=info
```

4. Invoke the test entrypoint:

```bash
golem worker invoke test:tts/debug synthesize
```

### Provider presets

The test manifest includes the following preset names:

- `elevenlabs-debug` / `elevenlabs-release`
- `polly-debug` / `polly-release`
- `google-debug` / `google-release`
- `deepgram-debug` / `deepgram-release`

Use the preset that matches the provider you want to validate.

### WIT interface reference

The TTS interface is defined in [`tts/tts/wit/golem-tts.wit`](tts/tts/wit/golem-tts.wit). The
following example illustrates the core synthesis flow:

```wit
use types.{text-input, tts-error, synthesis-result};

interface synthesis {
  synthesize: func(
    input: text-input,
    voice-id: string,
    options: option<synthesis-options>
  ) -> result<synthesis-result, tts-error>;
}
```

# golem-tts

WebAssembly Components providing a unified API for various Text-to-Speech (TTS) providers.

## Versions

Each TTS provider has its own implementation. Currently supported: **ElevenLabs** and **AWS Polly**.

| Name                          | Description                                                                                |
|-------------------------------|--------------------------------------------------------------------------------------------|
| `golem-tts-elevenlabs.wasm`    | TTS implementation for ElevenLabs, using custom Golem specific durability features |
| `golem-tts-aws.wasm`           | TTS implementation for AWS Polly, using custom Golem specific durability features |

Every component **exports** the same `golem:tts` interface, [defined here](wit/golem-tts.wit).

## Usage

### Environment Variables

Each provider has to be configured with connection details passed as environment variables:

| Provider   | Environment Variables |
|------------|----------------------|
| ElevenLabs | `ELEVENLABS_API_KEY` |
| AWS        | `AWS_ACCESS_KEY`, `AWS_SECRET_KEY`, `AWS_REGION` |

Additionally, the following environment variables can be used to configure the TTS behavior:

- `TTS_PROVIDER_LOG_LEVEL` - Set logging level (trace, debug, info, warn, error)
- `TTS_PROVIDER_MAX_RETRIES` - Maximum number of retries for failed requests (default: 5)

## Features

The TTS interface supports comprehensive text-to-speech functionality including:

### Supported Operations
- **Text-to-Speech Synthesis**: Convert text to high-quality audio streams.
- **Voice Listing**: Retrieve available voices from the provider.
- **Durability**: All operations are replay-safe within the Golem worker lifecycle.

### Planned Providers
- Azure Neural TTS
- Google Cloud Text-to-Speech
- OpenAI TTS

## Architecture

This project follows the official Golem AI architecture, ensuring modularity and 100% isomorphism with other Golem AI components like `golem-stt`.

![Architecture Diagram](https://raw.githubusercontent.com/youngWM/golem-ai/feat/golem-tts-implementation/bounties/009_Golem_TTS/repo_mirror/tts/architecture.png)

# golem-stt

WebAssembly Components providing a unified API for various Speech-to-Text (STT) providers.

## Versions

Each STT provider has two versions: **Default** (with Golem-specific durability features) and **Portable** (no Golem dependencies).

There are 10 published WASM files for each release:

| Name                                 | Description                                                                                |
|--------------------------------------|--------------------------------------------------------------------------------------------|
| `golem-stt-aws.wasm`                | STT implementation for AWS Transcribe, using custom Golem specific durability features |
| `golem-stt-azure.wasm`              | STT implementation for Azure Speech Services, using custom Golem specific durability features |
| `golem-stt-deepgram.wasm`           | STT implementation for Deepgram, using custom Golem specific durability features |
| `golem-stt-google.wasm`             | STT implementation for Google Cloud Speech-to-Text, using custom Golem specific durability features |
| `golem-stt-whisper.wasm`            | STT implementation for OpenAI Whisper, using custom Golem specific durability features |
| `golem-stt-aws-portable.wasm`       | STT implementation for AWS Transcribe, with no Golem specific dependencies |
| `golem-stt-azure-portable.wasm`     | STT implementation for Azure Speech Services, with no Golem specific dependencies |
| `golem-stt-deepgram-portable.wasm`  | STT implementation for Deepgram, with no Golem specific dependencies |
| `golem-stt-google-portable.wasm`    | STT implementation for Google Cloud Speech-to-Text, with no Golem specific dependencies |
| `golem-stt-whisper-portable.wasm`   | STT implementation for OpenAI Whisper, with no Golem specific dependencies |

Every component **exports** the same `golem:stt` interface, [defined here](wit/golem-stt.wit).

## Usage

For general usage information, integration examples, and getting started guides, see the [main README](../README.md).

### Environment Variables

Each provider has to be configured with connection details passed as environment variables:

| Provider  | Environment Variables |
|-----------|----------------------|
| AWS       | `AWS_REGION`, `AWS_ACCESS_KEY`, `AWS_SECRET_KEY`, `AWS_BUCKET_NAME` |
| Azure     | `AZURE_REGION`, `AZURE_SUBSCRIPTION_KEY` |
| Deepgram  | `DEEPGRAM_API_TOKEN`, `DEEPGRAM_ENDPOINT` (optional) |
| Google    | `GOOGLE_LOCATION`, `GOOGLE_BUCKET_NAME`, and either `GOOGLE_APPLICATION_CREDENTIALS` or (`GOOGLE_PROJECT_ID`, `GOOGLE_CLIENT_EMAIL`, `GOOGLE_PRIVATE_KEY`) |
| Whisper   | `OPENAI_API_KEY` |

Additionally, the following environment variables can be used to configure the STT behavior:

- `STT_PROVIDER_LOG_LEVEL` - Set logging level (trace, debug, info, warn, error)
- `STT_PROVIDER_MAX_RETRIES` - Maximum number of retries for failed requests (default: 10)

**Note**: For Google Cloud, you can use either the `GOOGLE_APPLICATION_CREDENTIALS` environment variable pointing to a JSON service account key file, or provide the individual credentials (`GOOGLE_PROJECT_ID`, `GOOGLE_CLIENT_EMAIL`, `GOOGLE_PRIVATE_KEY`). For Deepgram, the `DEEPGRAM_ENDPOINT` is optional and defaults to `https://api.deepgram.com/v1/listen`.

## Features

The STT interface supports comprehensive speech-to-text functionality including:

### Audio Formats
- WAV
- MP3
- FLAC
- OGG
- AAC
- PCM

### Advanced Features
- **Speaker Diarization** - Identify and separate different speakers
- **Custom Vocabulary** - Boost recognition of specific phrases
- **Multi-channel Audio** - Process stereo or multi-channel recordings
- **Timing Information** - Get precise start/end times for words and segments
- **Confidence Scores** - Receive confidence ratings for transcription accuracy
- **Language Detection** - Support for multiple languages (varies by provider)

## Examples

Take the [test application](../test/stt/components-rust/test-stt/src/lib.rs) as an example of using `golem-stt` from Rust. The implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test_transcribe`       | Simple single audio file transcription with basic configuration                    |
| `test_transcribe_many`  | Batch transcription of multiple audio files                                        |
| `test_list_supported_languages` | List all languages supported by the selected provider                      |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem` binary started with `golem server run`.

Then build and deploy the _test application_. The following profiles are available for testing:

| Profile Name         | Description                                                                           |
|----------------------|---------------------------------------------------------------------------------------|
| `aws-debug`          | Uses the AWS Transcribe implementation and compiles the code in debug profile        |
| `azure-debug`        | Uses the Azure Speech Services implementation and compiles the code in debug profile |
| `deepgram-debug`     | Uses the Deepgram implementation and compiles the code in debug profile              |
| `google-debug`       | Uses the Google Cloud Speech-to-Text implementation and compiles the code in debug profile |
| `whisper-debug`      | Uses the OpenAI Whisper implementation and compiles the code in debug profile        |

```bash
cd ../test/stt
golem app build -b whisper-debug
golem app deploy -b whisper-debug

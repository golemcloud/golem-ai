# Azure Speech-to-Text Component

Azure Speech-to-Text for WebAssembly and Golem Cloud.

## Key Notes

- Streaming API has been removed. Use multi-transcribe to run multiple batch jobs concurrently and return all results together.
- Languages list is a snapshot; see `stt/azure/src/languages.rs`.
- Durability: when the `durability` feature is enabled, minimal hooks record request lifecycle metadata (request id, success, durations).

## Environment Variables

- AZURE_SPEECH_KEY
- AZURE_SPEECH_REGION
- STT_PROVIDER_ENDPOINT
- STT_PROVIDER_TIMEOUT
- STT_PROVIDER_MAX_RETRIES
- STT_BUFFER_LIMIT_BYTES
- STT_PROVIDER_LOG_LEVEL
- STT_MAX_CONCURRENCY (native only; default 8) — bounds concurrent requests in multi-transcribe

## Options Mapping

| Option | Azure mapping |
|---|---|
| enable-timestamps | `wordLevelTimestamps=true` and parse words into `WordSegment` |
| enable-speaker-diarization | `diarizationEnabled=true`; populate `speaker_id` when present |
| language | `language=...` |
| model | forwarded as `deploymentId=...` when provided |
| profanity-filter | `profanityFilter=true/false` |
| speech-context | merged/deduped into `phraseList=a,b,c` |
| enable-word-confidence | `wordLevelConfidence=true`; parsed into `WordSegment.confidence` |
| enable-timing-detail | not used (Azure detailed format always returned) |

## Build

```bash
cargo component build -p golem-stt-azure
```

If you don't have cargo-component:

```bash
cargo install cargo-component
cargo component build -p golem-stt-azure
```

## Tests

```bash
cargo test -p golem-stt-azure

Env-guarded integration tests use the following (tests skip when unset):

- AZURE_SPEECH_KEY, AZURE_SPEECH_REGION
- AZURE_STT_TEST_AUDIO (wav), AZURE_STT_TEST_AUDIO_DIAR (wav multi-speaker)
- AZURE_STT_TEST_AUDIO_MP3, AZURE_STT_TEST_AUDIO_FLAC
- Optional: AZURE_STT_TEST_AUDIO_SILENCE, AZURE_STT_TEST_AUDIO_LONG
```




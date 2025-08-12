# Golem STT Components

This directory contains production-ready WASM components implementing the `golem:stt` interface for multiple speech-to-text providers. Each component is designed to be durable, fault-tolerant, and follows Golem conventions for component development.

## Overview

The `golem:stt` interface provides a unified abstraction over transcription functionality, enabling developers to interact with a common API regardless of provider differences. The interface supports:

- **Batch and streaming transcription**
- **Word-level timestamps**
- **Speaker diarization**
- **Custom vocabularies**
- **Confidence scores**
- **Graceful degradation** when providers don't support specific features

##  API Implementation

All STT components use **real provider APIs** and do not support mock/configurable endpoints:

- **Google**: `https://speech.googleapis.com/v1p1beta1/speech:recognize`
- **Azure**: `https://{region}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1`
- **AWS**: `https://transcribe.{region}.amazonaws.com` (with S3 integration)
- **Deepgram**: `https://api.deepgram.com`
- **OpenAI Whisper**: `https://api.openai.com/v1/audio/transcriptions`

The `STT_PROVIDER_ENDPOINT` environment variable has been removed to ensure production readiness and prevent accidental use of mock endpoints.


## Architecture

### Core Components

- **`stt/stt/`** - Shared library with common functionality
  - Configuration management
  - Durability integration
  - Error handling
  - HTTP client with retry logic
  - Streaming abstractions

- **`stt/{provider}/`** - Provider-specific implementations
  - WIT interface implementation
  - Provider-specific client logic
  - Error mapping
  - Streaming support (where available)

- **`test/stt/`** - Comprehensive test suite
  - Unit tests for all components
  - Integration tests
  - Streaming tests
  - Edge case handling
  - Performance tests

### WIT Interface

Each component exports the following WIT interfaces:

```wit
export golem:stt/types@1.0.0;
export golem:stt/vocabularies@1.0.0;
export golem:stt/languages@1.0.0;
export golem:stt/transcription@1.0.0;
```

## Configuration

### Common Environment Variables

All providers support these common configuration options:

```bash
# Endpoint configuration
STT_PROVIDER_ENDPOINT=https://api.provider.com

# Timeout and retry configuration
STT_PROVIDER_TIMEOUT=30          # Request timeout in seconds (default: 30)
STT_PROVIDER_MAX_RETRIES=3       # Maximum retry attempts (default: 3)

# Logging configuration
STT_PROVIDER_LOG_LEVEL=info      # trace|debug|info|warn|error|off
```

### Provider-Specific Configuration

#### Deepgram
```bash
DEEPGRAM_API_KEY=your_api_key
```

#### Azure Speech
```bash
AZURE_SPEECH_KEY=your_subscription_key
AZURE_SPEECH_REGION=eastus
```

#### AWS Transcribe
```bash
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
AWS_SESSION_TOKEN=your_session_token  # Optional, for temporary credentials
```

#### Google Cloud Speech-to-Text
```bash
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
GOOGLE_CLOUD_PROJECT=your_project_id
GOOGLE_ACCESS_TOKEN=your_oauth_token  # Alternative to service account
```

#### OpenAI Whisper
```bash
OPENAI_API_KEY=your_api_key
WHISPERX_ENDPOINT=https://whisperx.api.com  # Optional, for word-level timestamps
```

## Features & Capabilities

### Audio Format Support

All providers support common audio formats:

- **WAV** (PCM, various sample rates)
- **MP3** (various bitrates)
- **FLAC** (lossless compression)
- **OGG** (Ogg Vorbis)
- **AAC** (Advanced Audio Coding)
- **PCM** (raw audio data)

### Transcription Options

```rust
TranscribeOptions {
    enable_timestamps: Option<bool>,        // Word-level timing
    enable_speaker_diarization: Option<bool>, // Speaker identification
    language: Option<String>,               // Language code (e.g., "en-US")
    model: Option<String>,                  // Provider-specific model
    profanity_filter: Option<bool>,         // Content filtering
    vocabulary: Option<Vocabulary>,         // Custom vocabulary
    speech_context: Option<Vec<String>>,    // Context phrases
    enable_word_confidence: Option<bool>,   // Per-word confidence scores
    enable_timing_detail: Option<bool>,     // Detailed timing information
}
```

### Streaming Transcription

Providers that support streaming expose:

```rust
// Create streaming session
let stream = transcribe_stream(config, options)?;

// Send audio chunks
stream.send_audio(chunk)?;

// Signal end of audio
stream.finish()?;

// Receive results
while let Some(alternative) = stream.receive_alternative()? {
    println!("Partial result: {}", alternative.text);
}

// Clean up
stream.close();
```

### Error Handling

Comprehensive error mapping covers all common scenarios:

```rust
enum SttError {
    InvalidAudio(String),           // Corrupted or invalid audio
    UnsupportedFormat(String),      // Audio format not supported
    UnsupportedLanguage(String),    // Language not supported
    TranscriptionFailed(String),    // Processing failed
    Unauthorized(String),           // Invalid credentials
    AccessDenied(String),          // Permission denied
    QuotaExceeded(QuotaInfo),      // Rate/quota limits
    RateLimited(u32),              // Rate limiting (retry after seconds)
    InsufficientCredits,           // Account balance/credits
    UnsupportedOperation(String),   // Feature not supported
    ServiceUnavailable(String),     // Provider service issues
    NetworkError(String),          // Connectivity problems
    InternalError(String),         // Internal processing errors
}
```

## Durability & Caching

All components integrate with Golem's durability APIs for:

- **Request caching** - Avoid duplicate processing
- **Streaming state** - Resume interrupted streams
- **Vocabulary storage** - Persist custom vocabularies
- **Configuration caching** - Optimize repeated requests

### Caching Strategy

```rust
// Request-level caching
let request_key = make_request_key(&audio, &options);
if let Some(cached_result) = durable_store.get(&request_key) {
    return Ok(cached_result);
}

// Process and cache result
let result = provider.transcribe(audio, config, options)?;
durable_store.put(&request_key, &result);
```

## Graceful Degradation

The interface is designed to gracefully handle provider limitations:

### Whisper Degradation Examples

```rust
// Streaming not supported
transcribe_stream(config, options) 
    -> Err(SttError::UnsupportedOperation("Streaming not supported"))

// Speaker diarization not available
TranscriptAlternative {
    words: vec![WordSegment {
        speaker_id: None,  // Gracefully degraded
        confidence: None,  // Not provided by Whisper
        // ... other fields
    }]
}
```

### Feature Detection

```rust
// Check provider capabilities
match provider {
    "whisper" => {
        // Handle limitations
        assert!(!supports_streaming);
        assert!(!supports_diarization);
        assert!(!supports_word_confidence);
    },
    _ => {
        // Full feature support
    }
}
```

## Building & Testing

### Build All Components

```bash
# Build all WASM components
cargo component build --release --target wasm32-wasi-preview2

# Build specific provider
cargo component build -p stt-deepgram --release
```

### Run Tests

```bash
# Run all tests
cargo test --workspace --all-features

# Run provider-specific tests
cargo test -p stt-deepgram

# Run integration tests
cargo test --test integration

# Run streaming tests
cargo test --test streaming
```

### Test Coverage

The test suite covers all requirements:

- ✅ **Basic transcription** for all audio formats
- ✅ **Word-level timing and confidence** (where supported)
- ✅ **Speaker diarization** (where supported)
- ✅ **Streaming transcription** (where supported)
- ✅ **Error mappings** for all error scenarios
- ✅ **Edge cases** (silence, overlapping speakers, long audio)
- ✅ **Quota behavior** (real and simulated)
- ✅ **Durability integration** with Golem APIs

## Performance & Scalability

### Timeout & Retry Configuration

```bash
# Conservative settings for unreliable networks
STT_PROVIDER_TIMEOUT=60
STT_PROVIDER_MAX_RETRIES=5

# Aggressive settings for low-latency requirements
STT_PROVIDER_TIMEOUT=10
STT_PROVIDER_MAX_RETRIES=1
```

### Memory & Resource Management

- **Streaming buffers** are bounded to prevent memory exhaustion
- **Request caching** uses efficient checksumming
- **Connection pooling** for HTTP clients
- **Graceful cleanup** of resources

### Monitoring & Observability

```bash
# Enable detailed logging
STT_PROVIDER_LOG_LEVEL=debug

# Trace all requests
STT_PROVIDER_LOG_LEVEL=trace
```

## Production Deployment

### Security Considerations

- **API keys** are validated at startup
- **Input validation** prevents malicious audio
- **Rate limiting** respects provider quotas
- **Error messages** don't leak sensitive information

### High Availability

- **Automatic retries** with exponential backoff
- **Circuit breaker** patterns for failing providers
- **Graceful degradation** when features unavailable
- **Health checks** for provider endpoints

### Monitoring

Key metrics to monitor:

- Request success/failure rates
- Response times and latencies
- Quota usage and limits
- Cache hit rates
- Error distributions

## Testing

The test suite provides comprehensive coverage meeting all original specification requirements:

### Test Categories
- **Basic Functionality**: Batch/streaming transcription, language support, metadata validation
- **Edge Cases**: Silence handling, overlapping speakers, long audio, invalid formats
- **Network Resilience**: Error handling, rate limiting, quota behavior
- **Provider Integration**: Real API testing when credentials are available
- **Durability**: golem-rust integration testing with feature flags

### Comprehensive Test Coverage
-  **Basic transcription** for common formats (WAV, MP3, FLAC, OGG, AAC, PCM)
-  **Word-level timing and confidence** (where supported by provider)
-  **Speaker diarization** (AWS, Azure, Google, Deepgram; gracefully degraded for Whisper)
-  **Streaming transcription** (emulated HTTP gateway for AWS/Azure/Google/Deepgram; UnsupportedOperation for Whisper)
-  **Error mappings** for network issues, rate limits, auth failures, quota exceeded
-  **Edge cases** including silence detection, overlapping speakers, very long audio files
-  **Quota behavior** testing with proper error responses and graceful degradation
-  **Durability integration** with golem-rust APIs behind feature flag

### Running Tests

```bash
# Quick test run
cd test/stt
golem-cli app build
golem-cli app invoke --component test:stt --function test_batch

# Comprehensive test suite
./run_comprehensive_tests.sh

# Test specific categories
./run_comprehensive_tests.sh basic      # Basic functionality
./run_comprehensive_tests.sh edge       # Edge cases
./run_comprehensive_tests.sh integration # Provider integration
```

### Test Documentation
See `test/stt/TEST_COVERAGE.md` for detailed test coverage documentation and compliance matrix.

## Contributing

### Adding New Providers

1. Create provider directory: `stt/new-provider/`
2. Implement WIT interfaces in `src/component.rs`
3. Add provider-specific client in `src/client.rs`
4. Implement error mapping in `src/conversions.rs`
5. Add streaming support in `src/stream.rs` (if supported)
6. Update configuration in shared `config.rs`
7. Add comprehensive tests
8. Update documentation

### Code Quality Standards

- **100% test coverage** for all new code
- **Comprehensive error handling** for all scenarios
- **Graceful degradation** for unsupported features
- **Performance benchmarks** for critical paths
- **Security review** for credential handling

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For issues and questions:

- 🐛 **Bug reports**: Create an issue with reproduction steps
- 💡 **Feature requests**: Describe the use case and requirements
- 📚 **Documentation**: Improvements and clarifications welcome
- 🔧 **Provider issues**: Include provider logs and configuration
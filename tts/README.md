# Golem TTS (Text-to-Speech) Components

This directory contains TTS provider implementations for the Golem AI platform. Each provider is implemented as a separate WebAssembly component following the unified TTS interface defined in the WIT specification.

## Supported Providers

- **AWS Polly** - Amazon's cloud-based TTS service
- **Google Cloud TTS** - Google's Text-to-Speech API
- **ElevenLabs** - Advanced AI voice synthesis platform
- **Deepgram** - Real-time voice AI platform

## Feature Support Matrix

| Feature | AWS Polly | Google TTS | ElevenLabs | Deepgram |
|---------|-----------|------------|------------|----------|
| **Basic Synthesis** |
| Synthesize | ✅ | ✅ | ✅ | ✅ |
| Synthesize Batch | ✅ | ✅ | ✅ | ✅ |
| SSML Support | ✅ | ✅ | ✅ | ❌ |
| **Voice Management** |
| List Voices | ✅ | ✅ | ✅ | ✅ |
| Get Voice | ✅ | ✅ | ✅ | ✅ |
| List Languages | ✅ | ✅ | ✅ | ✅ |
| **Validation & Analysis** |
| Validate Input | ✅ | ✅ | ✅ | ✅ |
| Get Timing Marks | ❌ | ✅ | ❌ | ❌ |
| **Advanced Features** |
| Voice Cloning | ❌ | ❌ | ✅ | ❌ |
| Voice Design | ❌ | ❌ | ✅ | ❌ |
| Voice Conversion | ❌ | ❌ | ✅ | ❌ |
| Sound Effects | ❌ | ❌ | ✅ | ❌ |
| Pronunciation Lexicons | ✅ | ❌ | ✅ | ❌ |
| Long-Form Synthesis | ✅ | ❌ | ❌ | ❌ |

**Legend:**
- ✅ Supported
- ❌ Not Supported

## Directory Structure

```
tts/
├── polly/          # AWS Polly implementation
├── google/         # Google Cloud TTS implementation
├── elevenlabs/     # ElevenLabs implementation
├── deepgram/       # Deepgram implementation
├── tts/            # Core TTS library and traits
└── wit/            # WIT interface definitions
```

## Architecture

### Core Components

1. **TTS Interface (`tts/`)**: Defines the common `TtsClient` trait and shared utilities
2. **Provider Implementations**: Each provider directory contains:
   - `src/`: Rust implementation
   - `wit/`: WIT bindings
   - `Cargo.toml`: Dependencies

### Key Interfaces

The TTS system is organized into several WIT interfaces:

- **`voices`**: Voice discovery and management
- **`synthesis`**: Core text-to-speech operations
- **`advanced`**: Advanced features (voice cloning, design, lexicons, long-form)
- **`types`**: Common types and error definitions

## Getting Started

### Prerequisites

- Rust toolchain
- `cargo-component` for building WebAssembly components
- Provider-specific API credentials

### Building

Build all TTS components:
```bash
cd tts
cargo make build
```

Build a specific provider:
```bash
cd tts/polly
cargo component build --release
```

### Configuration

Each provider requires specific environment variables:

#### AWS Polly
```bash
AWS_ACCESS_KEY_ID=<your-access-key>
AWS_SECRET_ACCESS_KEY=<your-secret-key>
AWS_REGION=us-east-1  # Optional, defaults to us-east-1
AWS_S3_BUCKET=<bucket-name>  # Required for long-form synthesis
```

#### Google Cloud TTS
```bash
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
# OR
GOOGLE_SERVICE_ACCOUNT_JSON=<json-content>  # Alternative: pass JSON content directly
```

#### ElevenLabs
```bash
ELEVENLABS_API_KEY=<your-api-key>
```

#### Deepgram
```bash
DEEPGRAM_API_KEY=<your-api-key>
```

## Usage Examples

### Basic Synthesis

```rust
use golem_tts::synthesis::{synthesize, TextInput, TextType};
use golem_tts::voices::get_voice;

// Get a voice
let voice = get_voice("Danielle")?;

// Create text input
let input = TextInput {
    content: "Hello, world!".to_string(),
    text_type: TextType::Plain,
    language: Some("en-US".to_string()),
};

// Synthesize
let result = synthesize(&input, &voice, None)?;
```

### SSML Synthesis

```rust
let ssml_input = TextInput {
    content: r#"
        <speak>
            Hello! <break time="500ms"/>
            This is a test.
        </speak>
    "#.to_string(),
    text_type: TextType::Ssml,
    language: Some("en-US".to_string()),
};

let result = synthesize(&ssml_input, &voice, None)?;
```

### Long-Form Synthesis (AWS Polly)

```rust
use golem_tts::advanced::synthesize_long_form;

let operation = synthesize_long_form(
    long_text,
    voice,
    "s3://my-bucket/output/audio.mp3".to_string(),
    None,
)?;

// Poll for completion
loop {
    match operation.get_status() {
        OperationStatus::Completed => {
            let result = operation.get_result()?;
            break;
        }
        OperationStatus::Failed => {
            return Err("Synthesis failed");
        }
        _ => {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
```

### Pronunciation Lexicons (AWS Polly, ElevenLabs)

```rust
use golem_tts::advanced::{create_lexicon, PronunciationEntry};

let entries = vec![
    PronunciationEntry {
        word: "Golem".to_string(),
        pronunciation: "GOH-lem".to_string(),
        part_of_speech: Some("noun".to_string()),
    },
];

let lexicon = create_lexicon(
    "mylexicon".to_string(),
    "en-US",
    Some(&entries),
)?;
```

## Provider-Specific Notes

### AWS Polly

- **Long-Form Synthesis**: Requires S3 bucket for output storage
- **Lexicon Names**: Must be alphanumeric only (no hyphens or special characters), 1-20 characters
- **SSML**: Avoid non-ASCII characters in SSML content
- **Timing Marks**: Not supported without audio synthesis

### Google Cloud TTS

- **Authentication**: Supports both file-based (`GOOGLE_APPLICATION_CREDENTIALS`) and direct JSON content (`GOOGLE_SERVICE_ACCOUNT_JSON`)
- **SSML**: Full SSML support with extensive tag compatibility
- **Timing Marks**: Supported with audio synthesis
- **Long-Form Synthesis**: Not supported (API is in beta/v1beta1)
- **Lexicons**: Not supported
- **Voice Design**: Not supported
- **Voice Cloning**: Not supported
- **Voice Conversion**: Not supported
- **Sound Effects**: Not supported

### ElevenLabs

#### Supported Operations ✅

- **Basic Synthesis**:
  - `synthesize`: Standard text-to-speech synthesis
  - `synthesize_batch`: Batch synthesis of multiple texts
  - SSML support (limited to `<speak>` and `<break>` tags)

- **Voice Management**:
  - `list_voices`: List all available voices
  - `get_voice`: Get detailed voice information
  - `list_languages`: List supported languages

- **Validation**:
  - `validate_input`: Validate text input before synthesis

- **Advanced Features**:
  - `create_voice_clone`: Instant Voice Cloning (IVC) from audio samples
    - **Note**: Requires paid subscription tier
    - Creates voice clone from provided audio samples
  - `design_voice`: Professional Voice Cloning (PVC) with voice characteristics
    - **Note**: Requires paid subscription tier
    - Generates synthetic voice based on gender, age, accent, and personality traits
  - `convert_voice`: Voice-to-voice conversion
    - Converts audio from one voice to another
  - `generate_sound_effect`: AI-generated sound effects from text descriptions
    - Creates sound effects based on text prompts
  - **Pronunciation Lexicons**:
    - `create_lexicon`: Create custom pronunciation dictionaries
    - `get_lexicon`: Retrieve lexicon details
    - `list_lexicons`: List all lexicons
    - `update_lexicon`: Add/remove pronunciation rules
    - `delete_lexicon`: Remove lexicons

#### Unsupported Operations ❌

- **Long-Form Synthesis**: Not yet implemented
  - Returns `TtsError::UnsupportedOperation`
  - Use batch synthesis for longer texts as a workaround

- **Timing Marks**: Not supported
  - `get_timing_marks` returns `TtsError::UnsupportedOperation`

#### Subscription Requirements

**Free Tier**:
- ✅ Basic synthesis
- ✅ Voice listing and management
- ✅ Voice conversion
- ✅ Sound effects
- ✅ Pronunciation lexicons

**Paid Tier Required**:
- ⚠️ Voice cloning (`create_voice_clone`)
- ⚠️ Voice design (`design_voice`)

#### Known Limitations

- **API Rate Limits**: Free tier has strict rate limiting and abuse detection
- **SSML Support**: Limited to basic tags (`<speak>`, `<break>`)
- **Model Selection**: Uses default models; custom model selection not exposed
- **Long-Form**: Not implemented; use `synthesize_batch` for longer content

### Deepgram

- **SSML**: Not supported, plain text only
- **Real-time**: Optimized for low-latency synthesis
- **Limited Features**: Focuses on core synthesis functionality

## Testing

Run tests for a specific provider:
```bash
cd test/tts
make build
golem-cli component add --component-name test:tts target/wasm32-wasi/release/test_tts.wasm
golem-cli agent invoke-and-await --component test:tts --agent test-agent --function test:tts-exports/test-tts-api.{test1}
```

## Error Handling

All operations return `Result<T, TtsError>` where `TtsError` includes:

- `UnsupportedOperation`: Feature not supported by provider
- `RequestError`: Invalid request parameters
- `NetworkError`: Network/connectivity issues
- `RateLimited`: Rate limit exceeded
- `InternalError`: Provider-specific errors


# golem-embed

WebAssembly Components providing a unified API for various AI embedding and reranking providers (OpenAI, Cohere, Hugging Face, and VoyageAI).

## Versions

There are 8 published WASM files for each release:

| Name                                      | Description                                                                                |
|-------------------------------------------|--------------------------------------------------------------------------------------------|
| `golem-embed-openai.wasm`                 | Embedding implementation for OpenAI, using custom Golem specific durability features      |
| `golem-embed-cohere.wasm`                 | Embedding implementation for Cohere, using custom Golem specific durability features      |
| `golem-embed-hugging-face.wasm`           | Embedding implementation for Hugging Face, using custom Golem specific durability features|
| `golem-embed-voyageai.wasm`               | Embedding implementation for VoyageAI, using custom Golem specific durability features    |
| `golem-embed-openai-portable.wasm`        | Embedding implementation for OpenAI, with no Golem specific dependencies.                 |
| `golem-embed-cohere-portable.wasm`        | Embedding implementation for Cohere, with no Golem specific dependencies.                 |
| `golem-embed-hugging-face-portable.wasm`  | Embedding implementation for Hugging Face, with no Golem specific dependencies.           |
| `golem-embed-voyageai-portable.wasm`      | Embedding implementation for VoyageAI, with no Golem specific dependencies.               |

Every component **exports** the same `golem:embed` interface, [defined here](wit/golem-embed.wit).

The `-portable` versions only depend on `wasi:io`, `wasi:http` and `wasi:logging`.

The default versions also depend on [Golem's host API](https://learn.golem.cloud/golem-host-functions) to implement
advanced durability related features.

## Provider Capabilities

Each provider supports different functionality and input types:

| Provider      |Text Embedding | Image Embedding | Reranking |
|---------------|-----------|------|-------|
| OpenAI        | ✅   | ❌    | ❌        | 
| Cohere        | ✅   | ✅    | ✅        |
| Hugging Face  | ✅   | ❌    | ❌        | 
| VoyageAI      | ✅   | ❌    | ✅        | 


## Usage

Each provider has to be configured with an API key passed as an environment variable:

| Provider      | Environment Variable     |
|---------------|--------------------------|
| OpenAI        | `OPENAI_API_KEY`         |
| Cohere        | `COHERE_API_KEY`         |
| Hugging Face  | `HUGGING_FACE_API_KEY`   |
| VoyageAI      | `VOYAGEAI_API_KEY`       |

Additionally, setting the `GOLEM_EMBED_LOG=trace` environment variable enables trace logging for all the communication
with the underlying embedding provider.

### Code Examples

#### Basic Text Embedding

```rust
use golem::embed::embed::{self, Config, ContentPart, TaskType};

let config = Config {
    model: Some("text-embedding-3-small".to_string()),
    task_type: Some(TaskType::RetrievalDocument),
    dimensions: Some(1024),
    ..Default::default()
};

let response = embed::generate(
    &[ContentPart::Text("Your text here".to_string())],
    &config
)?;

println!("Vector: {:?}", response.embeddings[0].vector);
```

#### Document Reranking (Cohere/VoyageAI)

```rust
let query = "What is machine learning?";
let documents = vec![
    "Machine learning is AI".to_string(),
    "Weather is sunny".to_string(),
    "AI transforms industries".to_string(),
];

let response = embed::rerank(query, &documents, &config)?;
for result in response.results {
    println!("Doc {}: {:.3}", result.index, result.relevance_score);
}
```

#### Image Embedding (Cohere Only)

```rust
let response = embed::generate(
    &[ContentPart::Image(ImageUrl {
        url: "https://example.com/image.jpg".to_string()
    })],
    &config
)?;
```

### Using with Golem

#### Using a template

The easiest way to get started is to use one of the predefined **templates** Golem provides.

**NOT AVAILABLE YET**

#### Using a component dependency

To existing Golem applications the `golem-embed` WASM components can be added as a **binary dependency**.

**NOT AVAILABLE YET**

#### Integrating the composing step to the build

Currently it is necessary to manually add the [`wac`](https://github.com/bytecodealliance/wac) tool call to the
application manifest to link with the selected embedding implementation. The `test` directory of this repository shows an
example of this.

The summary of the steps to be done, assuming the component was created with `golem-cli component new rust my:example`:

1. Copy the `profiles` section from `common-rust/golem.yaml` to the component's `golem.yaml` file (for example in
   `components-rust/my-example/golem.yaml`) so it can be customized.
2. Add a second **build step** after the `cargo component build` which is calling `wac` to compose with the selected (
   and downloaded) `golem-embed` binary. See the example below.
3. Modify the `componentWasm` field to point to the composed WASM file.
4. Add the `golem-embed.wit` file (from this repository) to the application's root `wit/deps/golem:embed` directory.
5. Import `golem-embed.wit` in your component's WIT file: `import golem:embed/embed@1.0.0;'

Example app manifest build section:

```yaml
components:
  my:example:
    profiles:
      debug:
        build:
          - command: cargo component build
            sources:
              - src
              - wit-generated
              - ../../common-rust
            targets:
              - ../../target/wasm32-wasip1/debug/my_example.wasm
          - command: wac plug --plug ../../golem_embed_openai.wasm ../../target/wasm32-wasip1/debug/my_example.wasm -o ../../target/wasm32-wasip1/debug/my_example_plugged.wasm
            sources:
              - ../../target/wasm32-wasip1/debug/my_example.wasm
              - ../../golem_embed_openai.wasm
            targets:
              - ../../target/wasm32-wasip1/debug/my_example_plugged.wasm
        sourceWit: wit
        generatedWit: wit-generated
        componentWasm: ../../target/wasm32-wasip1/debug/my_example_plugged.wasm
        linkedWasm: ../../golem-temp/components/my_example_debug.wasm
        clean:
          - src/bindings.rs
```

### Using without Golem

To use the embedding provider components in a WebAssembly project independent of Golem you need to do the following:

1. Download one of the `-portable.wasm` versions
2. Download the `golem-embed.wit` WIT package and import it
3. Use [`wac`](https://github.com/bytecodealliance/wac) to compose your component with the selected embedding implementation.

## API Reference

### Core Functions

```rust
// Generate embeddings
generate(inputs: &[ContentPart], config: &Config) -> Result<EmbeddingResponse, Error>

// Rerank documents  
rerank(query: &str, documents: &[String], config: &Config) -> Result<RerankResponse, Error>
```

### Configuration

```rust
Config {
    model: Some("model-name".to_string()),           // Provider model
    task_type: Some(TaskType::RetrievalDocument),    // Use case optimization
    dimensions: Some(1024),                          // Output vector size
    truncation: Some(true),                          // Auto-truncate long inputs
    output_format: Some(OutputFormat::FloatArray),   // Vector format
    user: Some("user_id".to_string()),               // User tracking
    provider_options: vec![                          // Provider-specific options
        Kv { key: "key".to_string(), value: "value".to_string() }
    ],
}
```

### Task Types

- `RetrievalQuery` - Search queries
- `RetrievalDocument` - Documents to search
- `SemanticSimilarity` - Similarity tasks
- `Classification` - Text classification
- `Clustering` - Document grouping
- `QuestionAnswering` - Q&A systems
- `FactVerification` - Fact checking
- `CodeRetrieval` - Code search

### Response Types

```rust
struct EmbeddingResponse {
    embeddings: Vec<Embedding>,     // Vector embeddings
    usage: Option<Usage>,           // Token usage stats
    model: String,                  // Model used
    provider_metadata_json: Option<String>,
}

struct RerankResponse {
    results: Vec<RerankResult>,     // Ranked results
    usage: Option<Usage>,           // Token usage
    model: String,                  // Model used
    provider_metadata_json: Option<String>,
}

struct RerankResult {
    index: u32,                     // Original document index
    relevance_score: f32,           // Relevance score (0-1)
    document: Option<String>,       // Document text
}
```

### Error Handling

```rust
enum ErrorCode {
    InvalidRequest,        // Bad input
    ModelNotFound,         // Invalid model
    AuthenticationFailed,  // Invalid API key
    RateLimitExceeded,    // Rate limit hit
    ProviderError,        // Provider issue
    InternalError,        // Internal component error
    Unknown,              // Unclassified error
}

struct Error {
    code: ErrorCode,
    message: String,
    provider_error_json: Option<String>,
}
```

## Examples

Take the [test application](../test/embed/components-rust/test-embed/src/lib.rs) as an example of using `golem-embed` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Basic text embedding generation                                                           | 
| `test2`       | Document reranking functionality                                              |
| `test3`       | Image embedding (Cohere only)                                                            |
| `test4`       | Multi-modal embedding (text + image)                                                     |
| `test5`       | Default parameters usage                                                                  |
| `test6`       | Provider-specific options                                                                 |
| `test7`       | Advanced reranking with custom parameters                                                |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary
started with `golem server run`.

**NOTE**: `golem-embed` requires the latest (unstable) version of Golem currently. It's going to work with the next public
stable release 1.2.2.

Then build and deploy the _test application_. Select one of the following profiles to choose which provider to use:

| Profile Name | Provider | Compilation | Description |
|--------------|----------|-------------|-------------|
| `openai-debug` | OpenAI | Debug | Development with OpenAI |
| `openai-release` | OpenAI | Release | Production with OpenAI |
| `cohere-debug` | Cohere | Debug | Development with Cohere |
| `cohere-release` | Cohere | Release | Production with Cohere |
| `hugging-face-debug` | Hugging Face | Debug | Development with Hugging Face |
| `hugging-face-release` | Hugging Face | Release | Production with Hugging Face |
| `voyageai-debug` | VoyageAI | Debug | Development with VoyageAI |
| `voyageai-release` | VoyageAI | Release | Production with VoyageAI |

```bash
cd ../test/embed
golem app build -b openai-debug
golem app deploy -b openai-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the API key for the given provider:

```bash
golem worker new test:embed/debug --env OPENAI_API_KEY=xxx --env GOLEM_EMBED_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:embed/debug test1 --stream   # Basic text embedding
golem worker invoke test:embed/debug test2 --stream   # Document reranking
golem worker invoke test:embed/debug test3 --stream   # Image embedding (Cohere only)
golem worker invoke test:embed/debug test4 --stream   # Multi-modal (Cohere only)
golem worker invoke test:embed/debug test5 --stream   # Default parameters
golem worker invoke test:embed/debug test6 --stream   # Provider-specific options
golem worker invoke test:embed/debug test7 --stream   # Advanced reranking
```

## Development

This repository uses [cargo-make](https://github.com/sagiegurari/cargo-make) to automate build tasks.
Some of the important tasks are:

| Command                             | Description                                                                                            |
|-------------------------------------|--------------------------------------------------------------------------------------------------------|
| `cargo make build`                  | Build all components with Golem bindings in Debug                                                      |
| `cargo make release-build`          | Build all components with Golem bindings in Release                                                    |
| `cargo make build-portable`         | Build all components with no Golem bindings in Debug                                                   |
| `cargo make release-build-portable` | Build all components with no Golem bindings in Release                                                 |
| `cargo make unit-tests`             | Run all unit tests                                                                                     |
| `cargo make check`                  | Checks formatting and Clippy rules                                                                     |
| `cargo make fix`                    | Fixes formatting and Clippy rules                                                                      |
| `cargo make wit`                    | To be used after editing the `wit/golem-embed.wit` file - distributes the changes to all wit directories |

The `test` directory contains a **Golem application** for testing various features of the embedding components.
Check [the Golem documentation](https://learn.golem.cloud/quickstart) to learn how to install Golem and `golem-cli` to
run these tests.

## Resources

- [Golem Cloud Documentation](https://learn.golem.cloud/)
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WIT (WebAssembly Interface Types)](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [OpenAI Embeddings API](https://platform.openai.com/docs/guides/embeddings)
- [Cohere Embed API](https://docs.cohere.com/reference/embed)
- [Hugging Face Inference API](https://huggingface.co/docs/api-inference/index)
- [VoyageAI API Documentation](https://docs.voyageai.com/)
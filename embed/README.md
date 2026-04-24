# golem-embed

Rust libraries providing a unified API for various AI embedding and reranking providers, to be used with Golem.

## Versions

There are 4 published libraries for each release:

| Name                       | Description                                                                                |
|----------------------------|--------------------------------------------------------------------------------------------|
| `golem-ai-embed-openai`    | Embedding implementation for OpenAI, using custom Golem specific durability features      |
| `golem-ai-embed-cohere`    | Embedding implementation for Cohere, using custom Golem specific durability features      |
| `golem-ai-embed-hugging-face` | Embedding implementation for Hugging Face, using custom Golem specific durability features|
| `golem-ai-embed-voyageai`     | Embedding implementation for VoyageAI, using custom Golem specific durability features    |

Every library **exports** the same `golem:embed` interface.

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

### Using with Golem

#### Using a template

The easiest way to get started is to use one of the predefined **templates** Golem provides.

#### Using a dependency

To existing Golem applications the `golem-ai-embed` libraries can be added as a **dependency**
in the `Cargo.toml` file.

## Examples

Take the [test application](test/components-rust/test-embed/src/lib.rs) as an example of using `golem-embed` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Simple text embedding generation                                                           | 
| `test2`       | Demonstrates document reranking functionality                                              |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary
started with `golem server run`.

**NOTE**: `golem-embed` requires the latest (unstable) version of Golem currently. It's going to work with the next public
stable release 1.2.2.

Then build and deploy the _test application_. Select one of the following profiles to choose which provider to use:
| Profile Name | Description |
|--------------|-----------------------------------------------------------------------------------------------|
| `openai-debug` | Uses the OpenAI embedding implementation and compiles the code in debug profile |
| `openai-release` | Uses the OpenAI embedding implementation and compiles the code in release profile |
| `cohere-debug` | Uses the Cohere embedding implementation and compiles the code in debug profile |
| `cohere-release` | Uses the Cohere embedding implementation and compiles the code in release profile |
| `hugging-face-debug` | Uses the Hugging Face embedding implementation and compiles the code in debug profile |
| `hugging-face-release` | Uses the Hugging Face embedding implementation and compiles the code in release profile |
| `voyageai-debug` | Uses the VoyageAI embedding implementation and compiles the code in debug profile |
| `voyageai-release` | Uses the VoyageAI embedding implementation and compiles the code in release profile |

```bash
cd test
golem build --preset openai-debug
golem deploy --preset openai-debug --yes
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the API key for the given provider:

```bash
golem worker new test:embed/debug --env OPENAI_API_KEY=xxx --env GOLEM_EMBED_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:embed/debug test1 --stream 
```

## Development

This repository uses [cargo-make](https://github.com/sagiegurari/cargo-make) to automate build tasks.
Some of the important tasks are:

| Command                             | Description                                                                                            |
|-------------------------------------|--------------------------------------------------------------------------------------------------------|
| `cargo make build`                  | Build all libraries in Debug                                                                           |
| `cargo make release-build`          | Build all libraries in Release                                                                         |
| `cargo make unit-tests`             | Run all unit tests                                                                                     |
| `cargo make check`                  | Checks formatting and Clippy rules                                                                     |
| `cargo make fix`                    | Fixes formatting and Clippy rules                                                                      |

The `test` directory contains a **Golem application** for testing various features of the embedding libraries.
Check [the Golem documentation](https://learn.golem.cloud/quickstart) to learn how to install Golem and `golem-cli` to
run these tests.
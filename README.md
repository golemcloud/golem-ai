# Golem-AI

WebAssembly Components providing a unified API for various Ai features with multiple providers.

## LLM

There are 10 published WASM files for each LLM release:

| Name                                 | Description                                                                          |
|--------------------------------------|--------------------------------------------------------------------------------------|
| `golem-llm-anthropic.wasm`           | LLM implementation for Anthropic AI, using custom Golem specific durability features |
| `golem-llm-ollama.wasm`           | LLM implementation for Ollama, using custom Golem specific durability features |
| `golem-llm-grok.wasm`                | LLM implementation for xAI (Grok), using custom Golem specific durability features   |
| `golem-llm-openai.wasm`              | LLM implementation for OpenAI, using custom Golem specific durability features       |
| `golem-llm-openrouter.wasm`          | LLM implementation for OpenRouter, using custom Golem specific durability features   |
| `golem-llm-anthropic-portable.wasm`  | LLM implementation for Anthropic AI, with no Golem specific dependencies.            |
| `golem-llm-ollama-portable.wasm`  | LLM implementation for Ollama, with no Golem specific dependencies.            |
| `golem-llm-grok-portable.wasm`       | LLM implementation for xAI (Grok), with no Golem specific dependencies.              |
| `golem-llm-openai-portable.wasm`     | LLM implementation for OpenAI, with no Golem specific dependencies.                  |
| `golem-llm-openrouter-portable.wasm` | LLM implementation for OpenRouter, with no Golem specific dependencies.              |

Every component **exports** the same `golem:llm` interface, [defined here](llm/wit/golem-llm.wit).

The `-portable` versions only depend on `wasi:io`, `wasi:http` and `wasi:logging`.

The default versions also depend on [Golem's host API](https://learn.golem.cloud/golem-host-functions) to implement
advanced durability related features.

### LLM Usage

Each provider has to be configured with an API key passed as an environment variable:

| Provider   | Environment Variable |
|------------|----------------------|
| Anthropic  | `ANTHROPIC_API_KEY`  |
| Grok       | `XAI_API_KEY`        |
| OpenAI     | `OPENAI_API_KEY`     |
| OpenRouter | `OPENROUTER_API_KEY` |
| Ollama | `GOLEM_OLLAMA_BASE_URL` |

Additionally, setting the `GOLEM_LLM_LOG=trace` environment variable enables trace logging for all the communication
with the underlying LLM provider.

## Video

There are 8 published WASM files for each Video release:

| Name                                 | Description                                                                          |
|--------------------------------------|--------------------------------------------------------------------------------------|
| `golem-video-veo.wasm`           | Video implementation for VEO, using custom Golem specific durability features |
| `golem-video-veo-portable.wasm`           | Video implementation for VEO, with no Golem specific durability features |
| `golem-video-runway.wasm`          | Video implementation for Runway, using custom Golem specific durability features   |
| `golem-video-runway-portable.wasm`  | Video implementation for Runway, with no Golem specific dependencies.            |
| `golem-video-stability.wasm`  | Video implementation for Stability, with no Golem specific dependencies.            |
| `golem-video-stability-portable.wasm`       | Video implementation for Stability, with no Golem specific dependencies.              |
| `golem-video-kling.wasm`     | Video implementation for Kling, with no Golem specific dependencies.                  |
| `golem-video-kling-portable.wasm` | Video implementation for Kling, with no Golem specific dependencies.              |

Every component **exports** the same `golem:video` interface, [defined here](video/wit/golem-video.wit).

The default versions also depend on [Golem's host API](https://learn.golem.cloud/golem-host-functions) to implement
advanced durability related features.

### Video Usage

Each provider has to be configured with an API key passed as an environment variable:

| Provider    | Environment Variable                                      |
|-------------|-----------------------------------------------------------|
| VEO         | `VEO_PROJECT_ID`, `VEO_CLIENT_EMAIL`, `VEO_PRIVATE_KEY`   |
| Runway      | `RUNWAY_API_KEY`                                          |
| Stability   | `STABILITY_API_KEY`                                       |
| Kling       | `KLING_ACCESS_KEY`, `KLING_SECRET_KEY`                    |

**Note**:The VEO API Private Key needs to be passed as is, this includes the `-----BEGIN PRIVATE KEY-----` and `-----END PRIVATE KEY-----` lines, also including the newlines `\n`.

Additionally, setting the `GOLEM_VIDEO_LOG=trace` environment variable enables trace logging for all the communication
with the underlying video provider.

### Using with Golem

#### Using a template

The easiest way to get started is to use one of the predefined **templates** Golem provides.

**NOT AVAILABLE YET**

#### Using a component dependency

To existing Golem applications the `golem-llm` WASM components can be added as a **binary dependency**.

**NOT AVAILABLE YET**

#### Integrating the composing step to the build

Currently it is necessary to manually add the [`wac`](https://github.com/bytecodealliance/wac) tool call to the
application manifest to link with the selected LLM implementation. The `test` directory of this repository shows an
example of this.

The summary of the steps to be done, assuming the component was created with `golem-cli component new rust my:example`:

1. Copy the `profiles` section from `common-rust/golem.yaml` to the component's `golem.yaml` file (for example in
   `components-rust/my-example/golem.yaml`) so it can be customized.
2. Add a second **build step** after the `cargo component build` which is calling `wac` to compose with the selected (
   and downloaded) `golem-llm` binary. See the example below.
3. Modify the `componentWasm` field to point to the composed WASM file.
4. Add the `golem-llm.wit` file (from this repository) to the application's root `wit/deps/golem:llm` directory.
5. Import `golem-llm.wit` in your component's WIT file: `import golem:llm/llm@1.0.0;'

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
          - command: wac plug --plug ../../golem_llm_openai.wasm ../../target/wasm32-wasip1/debug/my_example.wasm -o ../../target/wasm32-wasip1/debug/my_example_plugged.wasm
            sources:
              - ../../target/wasm32-wasip1/debug/my_example.wasm
              - ../../golem_llm_openai.wasm
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

To use the LLM provider components in a WebAssembly project independent of Golem you need to do the following:

1. Download one of the `-portable.wasm` versions
2. Download the `golem-llm.wit` WIT package and import it
3. Use [`wac`](https://github.com/bytecodealliance/wac) to compose your component with the selected LLM implementation.

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
| `cargo make wit`                    | To be used after editing the `wit/golem-llm.wit` file - distributes the changes to all wit directories |

The `test` directory contains a **Golem application** for testing various features of the AI components.
Check [the Golem documentation](https://learn.golem.cloud/quickstart) to learn how to install Golem and `golem-cli` to
run these tests.


# golem-ai

WebAssembly Components providing API for modules LLM, WebSearch, Video and Search for various providers.

## Modules

This repository contains four modules, each with multiple provider implementations:

### LLM Module
Provides a unified API for various Large Language Model providers:
- **Anthropic** - Claude models via Anthropic API
- **OpenAI** - GPT models via OpenAI API  
- **OpenRouter** - Access to multiple models via OpenRouter
- **Amazon Bedrock** - AWS Bedrock models
- **Grok** - xAI's Grok models
- **Ollama** - Local models via Ollama

### WebSearch Module
Provides a unified API for various Web Search engines:
- **Brave** - Brave Search API
- **Google** - Google Custom Search API
- **Serper** - Serper.dev search API
- **Tavily** - Tavily AI search API

### Search Module
Provides a unified API for various Document Search engines:
- **Algolia** - Algolia search service
- **Elasticsearch** - Elasticsearch engine
- **Meilisearch** - Meilisearch engine
- **OpenSearch** - AWS OpenSearch
- **Typesense** - Typesense search engine

### Video Module
Provides a unified API for various Video Generation providers:
- **Veo** - Google's Veo video generation
- **Stability** - Stability AI video generation
- **Kling** - Kling video generation and lip-sync
- **Runway** - Runway ML video generation

## Component Versions

Each provider has two versions available:

### Default Versions
- Include Golem-specific durability features
- Depend on [Golem's host API](https://learn.golem.cloud/golem-host-functions)
- Provide advanced features like crash recovery and state persistence
- Example: `golem-llm-openai.wasm`

### Portable Versions  
- No Golem-specific dependencies
- Can be used in any WebAssembly environment
- Example: `golem-llm-openai-portable.wasm`

Every component **exports** the same unified interface for its module, defined in the respective WIT files:
- LLM: [`llm/wit/golem-llm.wit`](llm/wit/golem-llm.wit)
- WebSearch: [`websearch/wit/golem-web-search.wit`](websearch/wit/golem-web-search.wit)
- Search: [`search/wit/golem-search.wit`](search/wit/golem-search.wit)
- Video: [`video/wit/golem-video.wit`](video/wit/golem-video.wit)

For detailed information about each module and its providers, see the individual README files:
- [LLM Module](llm/README.md)
- [WebSearch Module](websearch/README.md)
- [Search Module](search/README.md)
- [Video Module](video/README.md)

## Using with Golem

### Using a template

The easiest way to get started is to use one of the predefined **templates** Golem provides.

**NOT AVAILABLE YET**

### Using a component dependency

To existing Golem applications the `golem-ai` WASM components can be added as a **binary dependency**.

**NOT AVAILABLE YET**

### Integrating the composing step to the build

Currently it is necessary to manually add the [`wac`](https://github.com/bytecodealliance/wac) tool call to the application manifest to link with the selected implementation. The `test` directory of this repository shows multiple examples of this.

The summary of the steps to be done, assuming the component was created with `golem-cli component new rust my:example`:

1. Copy the `profiles` section from `common-rust/golem.yaml` to the component's `golem.yaml` file (for example in `components-rust/my-example/golem.yaml`) so it can be customized.
2. Add a second **build step** after the `cargo component build` which is calling `wac` to compose with the selected (and downloaded) binary. See the example below.
3. Modify the `componentWasm` field to point to the composed WASM file.
4. Add the appropriate WIT file (from this repository) to the application's root `wit/deps/golem:<module>` directory.
5. Import the WIT file in your component's WIT file: `import golem:llm/llm@1.0.0;' (for LLM example)

Example app manifest build section (using OpenAI LLM as example):

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
For detailed information about available profiles and environment variables for each module, see the individual README files:
- [LLM Module](llm/README.md)
- [WebSearch Module](websearch/README.md)
- [Search Module](search/README.md)
- [Video Module](video/README.md)

### Using without Golem

To use the provider components in a WebAssembly project independent of Golem you need to do the following:

1. Download one of the `-portable.wasm` versions
2. Download the appropriate WIT package and import it
3. Use [`wac`](https://github.com/bytecodealliance/wac) to compose your component with the selected implementation.

## Examples

The `test` directory contains comprehensive examples for each module:

Individual test directories for each module (with examples):
- [LLM Test](test/llm/)
- [WebSearch Test](test/websearch/)
- [Search Test](test/search/)
- [Video Test](test/video/)
- [Video Advanced Test](test/video-advanced/)

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`

Binary start with `golem server run`.

Then build and deploy the _test application_. Select one of the available profiles to choose which provider to use. Profile names follow the pattern `<provider>-<build-type>` (e.g., `openai-debug`, `anthropic-release`, `brave-debug`, etc.).

Using example of `openai-debug` profile from LLM test, and respective environment variable:

```bash
cd test/llm
golem app build -b openai-debug
golem app deploy -b openai-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the ENVIRONMENT variable (eg.API key) for the given provider:

```bash
golem worker new test:llm/debug --env OPENAI_API_KEY=xxx --env GOLEM_LLM_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:llm/debug test1 --stream 
```

For detailed information about available profiles and environment variables for each module, and what tests are available, see the individual README files:
- [LLM Module](llm/README.md)
- [WebSearch Module](websearch/README.md)
- [Search Module](search/README.md)
- [Video Module](video/README.md)

## Development

This repository uses [cargo-make](https://github.com/sagiegurari/cargo-make) to automate build tasks.
Some of the important tasks are:

| Command                             | Description                                                                                                    |
|-------------------------------------|----------------------------------------------------------------------------------------------------------------|
| `cargo make build`                  | Build all components with Golem bindings in Debug                                                              |
| `cargo make release-build`          | Build all components with Golem bindings in Release                                                            |
| `cargo make build-portable`         | Build all components with no Golem bindings in Debug                                                           |
| `cargo make release-build-portable` | Build all components with no Golem bindings in Release                                                         |
| `cargo make unit-tests`             | Run all unit tests                                                                                             |
| `cargo make check`                  | Checks formatting and Clippy rules                                                                             |
| `cargo make fix`                    | Fixes formatting and Clippy rules                                                                              |
| `cargo make wit`                    | Used after editing the `<module>/wit/golem-<module>.wit` file - distributes the changes to all wit directories |
| `cargo make build-test-components`  | Builds all test apps in `/test`, with all provider build-options using `golem-cli app build -b <provider>`     |

**Note**: `cargo make` command build, release-build, build-portable, release-build-portable, wit, build-test-components, can be used with 
`cargo make <command> <module>` to target only the selected module. (e.g. `cargo make build llm`, `cargo make wit video`)

The `test` directory contains a **Golem application** for testing various features of the LLM, WebSearch, Video and Search components.
Check [the Golem documentation](https://learn.golem.cloud/quickstart) to learn how to install Golem and `golem-cli` to
run these tests.


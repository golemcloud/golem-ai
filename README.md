# golem-ai

Rust libraries providing API for modules LLM, WebSearch, Video and Search for various providers, to be used with Golem.

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

Every library **exports** the same unified interface for its module.

For detailed information about each module and its providers, see the individual README files:
- [LLM Module](llm/README.md)
- [WebSearch Module](websearch/README.md)
- [Search Module](search/README.md)
- [Video Module](video/README.md)

## Using with Golem

### Using a template

The easiest way to get started is to use one of the predefined **templates** Golem provides.

### Using a dependency

To existing Golem applications the `golem-ai` libraries can be added as a **dependency** in their `Cargo.toml` files.

For detailed information about available profiles and environment variables for each module, see the individual README files:
- [LLM Module](llm/README.md)
- [WebSearch Module](websearch/README.md)
- [Search Module](search/README.md)
- [Video Module](video/README.md)

## Examples

The `test` directory contains comprehensive examples for each module:

Individual test directories for each module (with examples):
- [LLM Test](test/llm/)
- [WebSearch Test](test/websearch/)
- [Search Test](test/search-old/)
- [Video Test](test/video/)
- [Video Advanced Test](test/video-advanced/)

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`

Binary start with `golem server run`.

Then build and deploy the _test application_. Select one of the available profiles to choose which provider to use. Profile names follow the pattern `<provider>-<build-type>` (e.g., `openai-debug`, `anthropic-release`, `brave-debug`, etc.).

Using example of `openai-debug` profile from LLM test, and respective environment variable:

```bash
cd test/llm
golem build --preset openai-debug
golem deploy --preset openai-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the ENVIRONMENT variable (eg.API key) for the given provider:

```bash
golem agent  new test:llm/debug --env OPENAI_API_KEY=xxx --env GOLEM_LLM_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem agent  invoke test:llm/debug test1 --stream 
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
| `cargo make build`                  | Build all libraries in Debug                                                                                   |
| `cargo make release-build`          | Build all libraries in Release                                                                                 |
| `cargo make unit-tests`             | Run all unit tests                                                                                             |
| `cargo make check`                  | Checks formatting and Clippy rules                                                                             |
| `cargo make fix`                    | Fixes formatting and Clippy rules                                                                              |
| `cargo make build-test`             | Builds all test apps in `/test`, with all provider build-options using `golem-cli build --preset <provider>`   |

**Note**: `cargo make` commands build, release-build, build-test, can be used with 
`cargo make <command> <module>` to target only the selected module. (e.g. `cargo make build llm`)

The `test` directory contains a **Golem application** for testing various features of the LLM, WebSearch, Video and Search libraries.
Check [the Golem documentation](https://learn.golem.cloud/quickstart) to learn how to install Golem and `golem-cli` to
run these tests.


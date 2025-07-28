# golem-web-search

WebAssembly Components providing a unified API for various web search providers.

## Versions

Each web search provider has two versions: **Default** (with Golem-specific durability features) and **Portable** (no Golem dependencies).

There are 8 published WASM files for each release:

| Name                                 | Description                                                                                |
|--------------------------------------|--------------------------------------------------------------------------------------------|
| `golem-web-search-brave.wasm`        | Web search implementation for Brave Search, using custom Golem specific durability features |
| `golem-web-search-google.wasm`       | Web search implementation for Google Custom Search, using custom Golem specific durability features |
| `golem-web-search-serper.wasm`       | Web search implementation for Serper.dev, using custom Golem specific durability features |
| `golem-web-search-tavily.wasm`       | Web search implementation for Tavily AI, using custom Golem specific durability features |
| `golem-web-search-brave-portable.wasm` | Web search implementation for Brave Search, with no Golem specific dependencies |
| `golem-web-search-google-portable.wasm` | Web search implementation for Google Custom Search, with no Golem specific dependencies |
| `golem-web-search-serper-portable.wasm` | Web search implementation for Serper.dev, with no Golem specific dependencies |
| `golem-web-search-tavily-portable.wasm` | Web search implementation for Tavily AI, with no Golem specific dependencies |

Every component **exports** the same `golem:web-search` interface, [defined here](wit/golem-web-search.wit).

## Usage

For general usage information, integration examples, and getting started guides, see the [main README](../README.md).

### Environment Variables

Each provider has to be configured with an API key passed as an environment variable:

| Provider | Environment Variable |
|----------|---------------------|
| Brave    | `BRAVE_API_KEY`     |
| Google   | `GOOGLE_API_KEY`, `GOOGLE_SEARCH_ENGINE_ID` |
| Serper   | `SERPER_API_KEY`    |
| Tavily   | `TAVILY_API_KEY`    |

Additionally, setting the `GOLEM_WEB_SEARCH_LOG=trace` environment variable enables trace logging for all the communication
with the underlying web search provider.

## Examples

Take the [test application](../test/websearch/components-rust/test-websearch/src/lib.rs) as an example of using `golem-web-search` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Simple, one-shot web search query                                                          | 
| `test2`       | Paginated search using search sessions with crash simulation                               |
| `test3`       | Time-filtered search for recent news                                                       |
| `test4`       | Domain filtering (include specific domains)                                                 |
| `test5`       | Domain exclusion and image inclusion                                                        |
| `test6`       | Multilingual search with specific region                                                    |
| `test7`       | Advanced search with high safe search and content chunks                                   |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary started with `golem server run`.

Then build and deploy the _test application_. The following profiles are available for testing:

| Profile Name         | Description                                                                           |
|----------------------|---------------------------------------------------------------------------------------|
| `brave-debug`        | Uses the Brave Search implementation and compiles the code in debug profile           |
| `brave-release`      | Uses the Brave Search implementation and compiles the code in release profile         |
| `google-debug`       | Uses the Google Custom Search implementation and compiles the code in debug profile   |
| `google-release`     | Uses the Google Custom Search implementation and compiles the code in release profile |
| `serper-debug`       | Uses the Serper.dev implementation and compiles the code in debug profile             |
| `serper-release`     | Uses the Serper.dev implementation and compiles the code in release profile           |
| `tavily-debug`       | Uses the Tavily AI implementation and compiles the code in debug profile             |
| `tavily-release`     | Uses the Tavily AI implementation and compiles the code in release profile           |

```bash
cd ../test/websearch
golem app build -b brave-debug
golem app deploy -b brave-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the ENVIRONMENT variable (eg.API key) for the given provider:

```bash
golem worker new test:websearch/debug --env BRAVE_API_KEY=xxx --env GOLEM_WEB_SEARCH_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:websearch/debug test1 --stream 
```


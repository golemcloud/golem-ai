# golem-search

WebAssembly Components providing a unified API for various search providers.

## Versions

Each search provider has two versions: **Default** (with Golem-specific durability features) and **Portable** (no Golem dependencies).

There are 10 published WASM files for each release:

| Name                                 | Description                                                                                |
|--------------------------------------|--------------------------------------------------------------------------------------------|
| `golem-search-algolia.wasm`          | Search implementation for Algolia, using custom Golem specific durability features |
| `golem-search-elasticsearch.wasm`    | Search implementation for Elasticsearch, using custom Golem specific durability features |
| `golem-search-meilisearch.wasm`      | Search implementation for Meilisearch, using custom Golem specific durability features |
| `golem-search-opensearch.wasm`       | Search implementation for OpenSearch, using custom Golem specific durability features |
| `golem-search-typesense.wasm`        | Search implementation for Typesense, using custom Golem specific durability features |
| `golem-search-algolia-portable.wasm` | Search implementation for Algolia, with no Golem specific dependencies |
| `golem-search-elasticsearch-portable.wasm` | Search implementation for Elasticsearch, with no Golem specific dependencies |
| `golem-search-meilisearch-portable.wasm` | Search implementation for Meilisearch, with no Golem specific dependencies |
| `golem-search-opensearch-portable.wasm` | Search implementation for OpenSearch, with no Golem specific dependencies |
| `golem-search-typesense-portable.wasm` | Search implementation for Typesense, with no Golem specific dependencies |

Every component **exports** the same `golem:search` interface, [defined here](wit/golem-search.wit).

## Usage

For general usage information, integration examples, and getting started guides, see the [main README](../README.md).

### Environment Variables

Each provider has to be configured with connection details passed as environment variables:

| Provider      | Environment Variables |
|---------------|----------------------|
| Algolia       | `ALGOLIA_APPLICATION_ID`, `ALGOLIA_API_KEY` |
| Elasticsearch | `ELASTICSEARCH_URL`, `ELASTICSEARCH_USERNAME`, `ELASTICSEARCH_PASSWORD`, `ELASTICSEARCH_API_KEY` |
| Meilisearch   | `MEILISEARCH_BASE_URL`, `MEILISEARCH_API_KEY` |
| OpenSearch    | `OPENSEARCH_BASE_URL`, `OPENSEARCH_USERNAME`, `OPENSEARCH_PASSWORD`, `OPENSEARCH_API_KEY` |
| Typesense     | `TYPESENSE_BASE_URL`, `TYPESENSE_API_KEY` |

Additionally, setting the `GOLEM_SEARCH_LOG=trace` environment variable enables trace logging for all the communication
with the underlying search provider.

**Note**: For Elasticsearch and OpenSearch, you can use either username/password authentication or API key authentication. If both are provided, API key takes precedence. For Meilisearch, the API key is optional and can be omitted if unauthenticated access is allowed.

## Examples

Take the [test application](../test/search/components-rust/test-search/src/lib.rs) as an example of using `golem-search` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Simple document insertion, retrieval, and deletion                                          | 
| `test2`       | Full-text search with basic queries and filters                                             |
| `test3`       | Search with sorting and pagination                                                          |
| `test4`       | Search with highlighting and facets                                                         |
| `test5`       | Schema inspection and validation                                                             |
| `test6`       | Streaming search behavior                                                                   |
| `test7`       | Error handling and edge cases                                                                |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary started with `golem server run`.

Then build and deploy the _test application_. The following profiles are available for testing:

| Profile Name         | Description                                                                           |
|----------------------|---------------------------------------------------------------------------------------|
| `algolia-debug`      | Uses the Algolia implementation and compiles the code in debug profile               |
| `algolia-release`    | Uses the Algolia implementation and compiles the code in release profile             |
| `elasticsearch-debug` | Uses the Elasticsearch implementation and compiles the code in debug profile         |
| `elasticsearch-release` | Uses the Elasticsearch implementation and compiles the code in release profile     |
| `meilisearch-debug`  | Uses the Meilisearch implementation and compiles the code in debug profile           |
| `meilisearch-release` | Uses the Meilisearch implementation and compiles the code in release profile       |
| `opensearch-debug`   | Uses the OpenSearch implementation and compiles the code in debug profile             |
| `opensearch-release` | Uses the OpenSearch implementation and compiles the code in release profile           |
| `typesense-debug`    | Uses the Typesense implementation and compiles the code in debug profile             |
| `typesense-release`  | Uses the Typesense implementation and compiles the code in release profile           |

```bash
cd ../test/search
golem app build -b algolia-debug
golem app deploy -b algolia-debug
```

Depending on the provider selected, environment variables have to be set for the worker to be started, containing the ENVIRONMENT variables (eg. connection details) for the given provider:

```bash
golem worker new test:search/debug --env ALGOLIA_APP_ID=xxx --env ALGOLIA_API_KEY=xxx --env GOLEM_SEARCH_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:search/debug test1 --stream 
```


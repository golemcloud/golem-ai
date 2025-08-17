# golem-vector

WebAssembly Components providing a unified API for various **vector database** providers.

## Versions

Each provider is released in two flavours:

* **Default** – contains Golem-specific durability integration giving per-operation consistency + replay.
* **Portable** – removes all Golem dependencies to run on any WASI 0.23 runtime.

| File name                            | Description |
|--------------------------------------|-------------|
| `golem-vector-qdrant.wasm`           | Qdrant implementation with durability |
| `golem-vector-pinecone.wasm`         | Pinecone implementation with durability |
| `golem-vector-milvus.wasm`           | Milvus implementation with durability |
| `golem-vector-pgvector.wasm`         | pgvector (PostgreSQL) implementation with durability |
| `golem-vector-qdrant-portable.wasm`  | Qdrant implementation **without** Golem dependencies |
| `golem-vector-pinecone-portable.wasm`| Pinecone implementation **without** Golem dependencies |
| `golem-vector-milvus-portable.wasm`  | Milvus implementation **without** Golem dependencies |
| `golem-vector-pgvector-portable.wasm`| pgvector implementation **without** Golem dependencies |

Every component **exports** the same `golem:vector` interface, [defined here](wit/golem-vector.wit).

## Usage

For general integration guidelines and getting-started instructions see the [top-level README](../README.md).

### Environment Variables

Each provider is configured through environment variables:

| Provider  | Required Environment Variables |
|-----------|--------------------------------|
| Qdrant    | `QDRANT_ENDPOINT`, `QDRANT_API_KEY` |
| Pinecone  | `PINECONE_ENDPOINT`, `PINECONE_API_KEY` |
| Milvus    | `MILVUS_ENDPOINT`, `MILVUS_API_KEY` |
| pgvector  | `PGVECTOR_URL` |

Set `GOLEM_VECTOR_LOG=trace` to enable detailed provider communication logs.

## Examples & Tests

Vector demos live under `test/vector/` and showcase:

* Creating collections and upserting vectors
* Similarity search with optional metadata filtering
* Provider-agnostic usage of namespaces & durability

Run the basic tests against a local Golem instance:

```bash
cd ../test/vector
# Build with qdrant provider in debug profile
golem app build -b qdrant-debug
# Deploy
golem app deploy -b qdrant-debug
# Start a worker with creds
golem worker new test:vector/debug --env QDRANT_ENDPOINT=http://localhost:6333 \
                                   --env QDRANT_API_KEY=none \
                                   --env GOLEM_VECTOR_LOG=trace
# Invoke the first test
golem worker invoke test:vector/debug test1 --stream
```

Check provider-specific README sections inside each crate for additional details and advanced operations.

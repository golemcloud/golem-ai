### Vector Qdrant Provider – Progress Checklist

- **Issue**: [golem-ai #21](https://github.com/golemcloud/golem-ai/issues/21)
- **Branch**: [`feat/vector-qdrant`](https://github.com/fjkiani/golem-ai/tree/feat/vector-qdrant)
- **PR (create)**: [open from branch](https://github.com/fjkiani/golem-ai/pull/new/feat/vector-qdrant)

### Phase 1: Scaffold + WIT
- [x] Scaffold `vector-qdrant` WASM component (WASI 0.2)
- [x] Vendor `golem:vector@1.0.0` WIT into:
  - [x] `vector-qdrant/wit/golem/vector/`
  - [x] `wit/golem/vector/`
- [x] Minimal `world.wit` (empty world) to allow compile without exports
- [x] Minimal `src/lib.rs` to compile cleanly
- [x] Build verification
  - Command: `cargo component build -p vector-qdrant`
  - Result: artifact at `target/wasm32-wasip1/debug/vector_qdrant.wasm`

### Phase 2: Wire Imports + Stubs
- [ ] Reintroduce imports in `vector-qdrant/wit/world.wit` for:
  - [ ] `golem:vector/collections@1.0.0`
  - [ ] `golem:vector/vectors@1.0.0`
  - [ ] `golem:vector/search@1.0.0`
- [ ] Generate bindings and add minimal stubs
  - [ ] Return `unsupported-feature` errors from all functions initially
  - [ ] Ensure component builds with exports

### Phase 3: Qdrant Adapter (MVP)
- [ ] Config via env vars (endpoint, API key)
- [ ] Collections: upsert, list, get, delete
- [ ] Vectors: upsert, get, update, delete, list, count
- [ ] Search: `search-vectors`, `find-similar`
- [ ] Error mapping to `vector-error`
- [ ] Durability: wrap ops using Golem durability APIs

### Phase 4: Extended Features (as time allows)
- [ ] Namespaces
- [ ] Search-extended (recommendations, groups, range)
- [ ] Analytics
- [ ] Connection interface (optional if using env-only config)

### Testing
- [ ] Unit tests: type mapping, request builders
- [ ] Integration: upsert/search round-trips (Qdrant test instance)
- [ ] Golem compatibility tests (1.2.x)

### Deliverables
- [ ] `vector-qdrant.wasm` (WASI 0.2)
- [ ] README with env config and usage
- [ ] CI job building the component

### Notes
- Some WIT toolchains are strict about recursive types. We’ll validate once imports/exports are wired and adjust if needed. 
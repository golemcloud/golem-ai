#!/usr/bin/env bash
set -euo pipefail

VERSION="0.5.1"
PUBLISH_FLAGS="--no-verify --allow-dirty"
RETRY_DELAY=60
MAX_RETRIES=5
SLEEP_BETWEEN=30

echo "=== Publishing golem-ai v${VERSION} ==="

# Step 1: Set version on all workspace crates
echo "Setting version to ${VERSION}..."
cargo set-version --workspace "${VERSION}"

# Step 2: Release build
echo "Running release build..."
cargo make release-build

# Helper: publish a single crate with retry on 429
publish_crate() {
  local crate_path="$1"
  local crate_name
  crate_name=$(grep '^name' "${crate_path}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')

  for attempt in $(seq 1 "${MAX_RETRIES}"); do
    echo "Publishing ${crate_name} (attempt ${attempt}/${MAX_RETRIES})..."
    set +e
    output=$(cargo publish -p "${crate_name}" ${PUBLISH_FLAGS} 2>&1)
    exit_code=$?
    set -e

    if [ ${exit_code} -eq 0 ]; then
      echo "  ✅ ${crate_name} published successfully"
      return 0
    fi

    # Already published — not an error
    if echo "${output}" | grep -q "already uploaded"; then
      echo "  ⏭️  ${crate_name} already published, skipping"
      return 0
    fi

    # Rate limited (429)
    if echo "${output}" | grep -qi "429\|rate limit\|try again\|too many requests"; then
      echo "  ⚠️  Rate limited on ${crate_name}, waiting ${RETRY_DELAY}s before retry..."
      sleep "${RETRY_DELAY}"
      # Increase delay for next retry
      RETRY_DELAY=$((RETRY_DELAY + 30))
      continue
    fi

    # Some other error
    echo "  ❌ Failed to publish ${crate_name}:"
    echo "${output}"
    return 1
  done

  echo "  ❌ Exhausted retries for ${crate_name}"
  return 1
}

# Core crates (no internal deps) — must be published first
CORE_CRATES=(
  llm/llm
  embed/embed
  websearch/websearch
  search/search
  graph/graph
  video/video
  exec/exec
  stt/stt
  tts/tts
  vector/vector
)

# Provider crates (depend on core crates)
PROVIDER_CRATES=(
  llm/anthropic
  llm/bedrock
  llm/grok
  llm/ollama
  llm/openai
  llm/openrouter
  embed/cohere
  embed/hugging-face
  embed/openai
  embed/voyageai
  websearch/brave
  websearch/google
  websearch/serper
  websearch/tavily
  search/algolia
  search/elasticsearch
  search/meilisearch
  search/opensearch
  search/typesense
  graph/arangodb
  graph/janusgraph
  graph/neo4j
  video/kling
  video/runway
  video/stability
  video/veo
  stt/aws
  stt/azure
  stt/deepgram
  stt/google
  stt/whisper
  tts/aws
  tts/deepgram
  tts/elevenlabs
  tts/google
  vector/milvus
  vector/pgvector
  vector/pinecone
  vector/qdrant
)

FAILED=()

echo ""
echo "=== Publishing core crates ==="
for crate in "${CORE_CRATES[@]}"; do
  RETRY_DELAY=60
  if ! publish_crate "${crate}"; then
    FAILED+=("${crate}")
  fi
  sleep "${SLEEP_BETWEEN}"
done

echo ""
echo "=== Publishing provider crates ==="
for crate in "${PROVIDER_CRATES[@]}"; do
  RETRY_DELAY=60
  if ! publish_crate "${crate}"; then
    FAILED+=("${crate}")
  fi
  sleep "${SLEEP_BETWEEN}"
done

echo ""
if [ ${#FAILED[@]} -eq 0 ]; then
  echo "🎉 All crates published successfully!"
else
  echo "❌ The following crates failed to publish:"
  for f in "${FAILED[@]}"; do
    echo "  - ${f}"
  done
  exit 1
fi

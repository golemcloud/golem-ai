#!/usr/bin/env bash
# Run from the test application's directory. Extra arguments are global Golem CLI options.
# The test must signal one GOLEM_AI_RECOVERY_READY checkpoint inside an incomplete
# custom durable operation. Crashes are injected externally: guest panics are terminal.
set -euo pipefail

agent=$1
method=$2
shift 2
output=$(mktemp)
pid=
cleanup() {
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    fi
    rm -f "$output"
}
trap cleanup EXIT

golem "$@" --format json agent invoke "$agent" "$method" >"$output" &
pid=$!
crashed=false
deadline=$((SECONDS + 600))
while kill -0 "$pid" 2>/dev/null; do
    if [[ "$crashed" == false ]] && grep -q '"message":"GOLEM_AI_RECOVERY_READY"' "$output"; then
        golem "$@" agent simulate-crash "$agent" >&2
        crashed=true
    fi
    if ((SECONDS >= deadline)); then
        cat "$output" >&2
        echo "Timed out waiting for $agent/$method to recover" >&2
        exit 1
    fi
    sleep 0.2
done

status=0
wait "$pid" || status=$?
pid=
cat "$output" >&2
if ((status != 0)); then
    exit "$status"
fi
if [[ "$crashed" != true ]]; then
    echo "$agent/$method finished without exercising recovery" >&2
    exit 1
fi
# Keep stdout compatible with `golem agent invoke --no-stream --format json`.
jq -ce 'select(."$type" == "agent.invoke")' "$output"

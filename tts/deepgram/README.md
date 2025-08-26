# tts-elevenlabs (golem:tts provider)

## Build
cargo component build --release -p tts-elevenlabs

## Smoke tests (WASI HTTP adapter required)
wasmtime run -S http \
  --env DEEPGRAM_API_KEY='YOUR_KEY' \
  --invoke 'health()' \
  target/wasm32-wasip1/release/tts_elevenlabs.wasm

VOICE_ID='21m00Tcm4TlvDq8ikWAM'
TEXT='Hello from ElevenLabs on Golem!'
wasmtime run -S http \
  --env DEEPGRAM_API_KEY='YOUR_KEY' \
  --invoke 'synth-b64("'"$VOICE_ID"'","'"$TEXT"'")' \
  target/wasm32-wasip1/release/tts_elevenlabs.wasm \
| python3 -c 'import sys,json,base64,re;s=sys.stdin.read().strip();m=re.match(r"^(ok|err)\((.*)\)$",s,re.S); \
  s=(json.loads(m.group(2)) if m and m.group(1)=="ok" else (json.loads(s) if s.startswith("\"") else s)); \
  sys.stdout.buffer.write(base64.b64decode("".join(s.split())))' > out.mp3

---
name: run-heedm
description: >
  Launch, drive, and verify the heedm app: dev run, screenshot for UI
  verification, whisper-server lifecycle (start/health/kill/orphans), and the
  record→transcribe STT smoke test. Use when asked to run/avvia the app,
  screenshot the UI, debug whisper-server, or verify a recording/STT feature
  end-to-end before declaring it done.
---

## Prerequisites

- Rust env: `source ~/.cargo/env` (Rust lives in `~/.cargo`).
- `src-tauri/binaries/whisper-server` must exist or `tauri dev` fails with
  `resource path 'binaries/whisper-server' doesn't exist`. If missing:
  `bash scripts/build-whisper-server.sh`.
- Screenshots from the terminal need the terminal app granted
  **System Settings → Privacy & Security → Screen Recording** (one-time, manual).
  Do NOT try osascript/System Events UI scripting — assistive access is denied
  on this machine and has failed in every past session.

## Launch (dev)

```sh
source ~/.cargo/env && npm run tauri dev
```

Run in background; first compile takes minutes, later runs seconds. Ready when
the log shows the Vite server + app window opens.

## Screenshot verification

```sh
screencapture -x /tmp/heedm-shot.png
```

Then Read the PNG. Full-screen capture — no window ID needed, no assistive
access. If it produces a black/empty image, the terminal lacks the Screen
Recording permission (see prerequisites); tell the user, don't retry.

## whisper-server lifecycle

The app manages the server itself (spawn on `start_local_server`, kill on app
exit). For manual debugging:

```sh
lsof -i :8080                          # who owns the port
curl -s http://127.0.0.1:8080/health   # server alive?
pkill -f whisper-server                # kill orphans (check lsof again after)
# manual start (default model dir):
src-tauri/binaries/whisper-server \
  --model "$HOME/Library/Application Support/com.vincenzomars.heedm/models/ggml-large-v3-turbo.bin" \
  --host 127.0.0.1 --port 8080
```

Orphan gotcha: a `whisper-server` still alive after app quit may predate the
current fix — check its start time (`ps -o lstart -p <pid>`) before concluding
the kill-on-exit code is broken.

## STT smoke test (record→transcribe)

Run this before declaring any recorder/STT feature done. Generates a spoken
fixture with native tools — no stored fixture file:

```sh
say -v Alice -o /tmp/heedm-fix.aiff "questa è una prova di trascrizione"
afconvert -f WAVE -d LEI16@16000 -c 1 /tmp/heedm-fix.aiff /tmp/heedm-fix.wav
curl -s http://127.0.0.1:8080/inference \
  -F file=@/tmp/heedm-fix.wav -F language=it -F response_format=verbose_json \
  | jq -e '.text | length > 0'
```

Pass = jq exits 0 and `.text` contains something like the spoken phrase.
Known failure modes (both shipped as bugs in the past):
- `{"error":"failed to read WAV file"}` → input not mono/16kHz/16-bit PCM
- 404 → wrong endpoint; whisper.cpp exposes only `/inference`, `/load`, `/health`
  (no OpenAI-style `/v1/audio/transcriptions`)

For a full-loop check, also record from the real app (user does this part —
TCC mic/screen permissions belong to the app, not the terminal) and transcribe
the produced WAV the same way.

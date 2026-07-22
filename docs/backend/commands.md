# Tauri Commands — reference

File: `src-tauri/src/commands.rs`
Registrati in: `src-tauri/src/lib.rs`

## Recording

| Command | Input | Output | Side effects |
|---|---|---|---|
| `start_recording` | — | `Result<(), String>` | Avvia stream cpal mic + SCStream sistema, popola buffer |
| `stop_recording` | — | `Result<String, String>` | Ferma stream, mix, crea cartella `recordings_dir/YYYY-MM-DD HH.MM.SS/`, scrive WAV come `recording.wav`; se l'audio di sistema era attivo, stima e scrive anche `recording.diarization.json` nella stessa cartella (best-effort, vedi `docs/backend/recorder.md`); ritorna path WAV |
| `get_recording_status` | — | `Result<RecordingStatus, String>` | — |
| `list_recordings` | — | `Result<Vec<RecordingEntry>, String>` | Scansiona `recordings_dir`, legge `transcript.json` da ogni sottocartella se presente; ordine cronologico inverso |

## Import

| Command | Input | Output | Side effects |
|---|---|---|---|
| `import_audio_file` | — | `Result<Option<String>, String>` | Apre il picker file (filtro `wav/mp3/m4a/mp4/flac/ogg/aac`), decodifica via symphonia, downmix mono + resample a 16kHz, scrive `recording.wav` in una nuova cartella `recordings_dir/YYYY-MM-DD HH.MM.SS/` (identico output a `stop_recording`, così l'entry segue lo stesso flusso Records/trascrizione); ritorna il path WAV, o `None` se annullato. Nessun sidecar diarizzazione (file singolo → speaker `None`) |

Formati compressi decodificati con **symphonia** (Rust puro, niente ffmpeg — coerente col resample puro-Rust già usato in `prepare_for_whisper`). mp4/m4a = solo traccia audio AAC.

### `RecordingStatus`
```rust
pub struct RecordingStatus {
    pub is_recording: bool,
    pub duration_ms: u64,
}
```

### `RecordingEntry`
```rust
pub struct RecordingEntry {
    pub folder_path: String,  // path assoluto cartella (es. .../Records/2024-06-28 14.30.45)
    pub name: String,         // nome cartella = timestamp
    pub wav_path: String,     // path assoluto recording.wav
    pub transcript: Option<TranscriptResult>,
}
```

## STT — Settings

| Command | Input | Output | Side effects |
|---|---|---|---|
| `get_stt_settings` | — | `SttSettings` | Legge `settings.json`, ritorna default se assente |
| `save_stt_settings` | `settings: SttSettings` | `Result<(), String>` | Scrive `settings.json` |
| `get_local_model_status` | — | `bool` | Controlla esistenza file su disco |
| `get_local_model_path` | — | `String` | Path assoluto del modello (rispetta `model_dir` se impostato); crea la cartella padre se assente |
| `get_recordings_dir` | — | `String` | Path assoluto cartella registrazioni corrente (rispetta `recordings_dir` se impostato) |
| `pick_directory` | — | `Option<String>` | Apre dialog "scegli cartella", `None` se annullato |

## STT — Download e server

| Command | Input | Output | Side effects |
|---|---|---|---|
| `download_local_model` | — | `Result<(), String>` | Scarica il modello (binary già bundled nell'app), emette eventi `download-progress`, aggiorna settings |
| `start_local_server` | — | `Result<(), String>` | Spawna whisper-server, attende fino a 60s |
| `start_stt_server` | — | `Result<(), String>` | Alias di `start_local_server` |
| `check_stt_server` | — | `String` | `"running"` o `"stopped"` |

## Permessi OS (macOS)

Wrapper sottili attorno a `permissions.rs` (CoreGraphics FFI + `open` di System
Settings) — vedi `docs/architecture.md` per il modulo. Su piattaforme diverse
da macOS gli stub ritornano sempre stato "concesso"/no-op.

| Command | Input | Output | Side effects |
|---|---|---|---|
| `check_screen_recording_permission` | — | `bool` | `CGPreflightScreenCaptureAccess` — sola lettura, nessun prompt |
| `request_screen_recording_permission` | — | `bool` | `CGRequestScreenCaptureAccess` — innesca il prompt di sistema se lo stato non è ancora deciso, altrimenti no-op |
| `open_permission_settings` | `pane: "microphone" \| "screen-recording"` | `Result<(), String>` | Apre il pannello Privacy & Security pertinente in System Settings via `open x-apple.systempreferences:...`; valore di `pane` non riconosciuto → `Err` (mai passato al comando di sistema) |

Nota: per il microfono non esiste un check di stato — l'unica API è
`AVCaptureDevice.authorizationStatus`, un metodo Objective-C che richiederebbe
bridging `objc2`/AVFoundation solo per un'indicazione di stato. Si espone solo
la CTA per aprire il pannello di sistema (vedi `docs/frontend/ui.md`).

## STT — Trascrizione

| Command | Input | Output | Side effects |
|---|---|---|---|
| `transcribe_recording` | `path: String` | `Result<TranscriptResult, String>` | Legge WAV, converte in memoria a 16kHz/mono/16-bit (`prepare_for_whisper`), POST `/inference` a whisper-server, poi — se esiste un sidecar `recording.diarization.json` nella stessa cartella — popola `TranscriptSegment.speaker` per sovrapposizione temporale (vedi `docs/backend/stt.md` → Diarizzazione); scrive `transcript.json` nella cartella del WAV (best-effort, `.ok()`) |

## Eventi emessi (Rust → Frontend)

| Evento | Payload | Quando |
|---|---|---|
| `download-progress` | `{ step: "model"\|"done", pct: number }` | Durante `download_local_model` |

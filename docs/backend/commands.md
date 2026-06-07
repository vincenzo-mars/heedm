# Tauri Commands — reference

File: `src-tauri/src/commands.rs`
Registrati in: `src-tauri/src/lib.rs`

## Recording

| Command | Input | Output | Side effects |
|---|---|---|---|
| `start_recording` | — | `Result<(), String>` | Avvia stream cpal mic + SCStream sistema, popola buffer |
| `stop_recording` | — | `Result<String, String>` | Ferma stream, mix, scrive WAV in `recordings_dir/Registrazione <timestamp leggibile>.wav`, ritorna path |
| `get_recording_status` | — | `Result<RecordingStatus, String>` | — |

### `RecordingStatus`
```rust
pub struct RecordingStatus {
    pub is_recording: bool,
    pub duration_ms: u64,
}
```

## STT — Settings

| Command | Input | Output | Side effects |
|---|---|---|---|
| `get_stt_settings` | — | `SttSettings` | Legge `settings.json`, ritorna default se assente |
| `save_stt_settings` | `settings: SttSettings` | `Result<(), String>` | Scrive `settings.json` |
| `get_local_model_status` | — | `bool` | Controlla esistenza file su disco |
| `get_local_model_path` | — | `String` | Path assoluto del modello (rispetta `model_dir` se impostato) |
| `get_recordings_dir` | — | `String` | Path assoluto cartella registrazioni corrente (rispetta `recordings_dir` se impostato) |
| `pick_directory` | — | `Option<String>` | Apre dialog "scegli cartella", `None` se annullato |

## STT — Download e server

| Command | Input | Output | Side effects |
|---|---|---|---|
| `download_local_model` | — | `Result<(), String>` | Scarica binary + modello, emette eventi `download-progress`, aggiorna settings |
| `start_local_server` | — | `Result<(), String>` | Spawna whisper-server, attende fino a 60s |
| `start_stt_server` | — | `Result<(), String>` | Alias di `start_local_server` |
| `check_stt_server` | — | `String` | `"running"` o `"stopped"` |

## STT — Trascrizione

| Command | Input | Output | Side effects |
|---|---|---|---|
| `transcribe_recording` | `path: String` | `Result<TranscriptResult, String>` | Legge file WAV, POST a whisper-server |

## Eventi emessi (Rust → Frontend)

| Evento | Payload | Quando |
|---|---|---|
| `download-progress` | `{ step: "binary"\|"model"\|"done", pct: number }` | Durante `download_local_model` |

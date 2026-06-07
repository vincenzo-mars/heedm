# Architettura

## Struttura repo

`src/` (frontend Svelte) + `src-tauri/` (backend Rust) a top-level: layout standard generato da `npm create tauri-app` / template ufficiali Tauri+Vite. `src-tauri/` deve restare a root perché Cargo.toml e config bundler usano path relativi fissi rispetto alla root del progetto. Layout monorepo (`apps/frontend` + `apps/backend`) avrebbe senso solo con più target/build separate da condividere — non è il caso qui (single-app desktop).

## Flusso dati end-to-end

```
[Microfono]         [Audio sistema]
    │                     │
    │ cpal input           │ ScreenCaptureKit (macOS)
    │                     │ PipeWire monitor (Linux)
    ▼                     ▼
[mic_samples: Arc<Mutex<Vec<f32>>>]   [sys_samples: Arc<Mutex<Vec<f32>>>]
                │                              │
                ├──────────────┬───────────────┤   (buffer grezzi, separati — usati
                │              │               │    due volte prima di essere fusi)
                ▼              │               ▼
       [mixer::mix()]         │      [diarization::estimate_timeline()]
       somma sample-by-sample │      energia RMS mic vs sistema a finestre
       clamp ±1.0             │      → intervalli {start, end, label}
                │             │               │
                ▼             │               ▼
       [writer::write_wav()] │      [<nome>.diarization.json]
       WAV Float32           │      sidecar accanto al WAV
       sample_rate/channels  │      (solo se audio sistema attivo,
                │            │       best-effort — non blocca la REC)
                ▼            │               │
       [file .wav su disco] ─┘               │
                │                            │
                ▼                            │
   [transcribe_recording() command]          │
   multipart POST /inference                 │
   language: it, response_format:            │
   verbose_json (prepare_for_whisper         │
   converte in memoria a 16kHz/mono)         │
                │                            │
                ▼                            │
   [whisper-server locale]                   │
   127.0.0.1:8080                            │
   modello: ggml-large-v3-turbo              │
                │                            │
                ▼                            │
   [TranscriptResult: text, segments[]] ◄────┘
   merge per sovrapposizione temporale →
   segments[].speaker = "mic"|"sys"|"both"|None
                │
                ▼
       [Frontend Svelte]
       TranscriptView — gruppi per speakerInfo() (Tu/Sistema/Sovrapposti/...)
```

## Componenti principali

| Layer | Tecnologia | Ruolo |
|---|---|---|
| Frontend | Svelte 5 + TypeScript | UI, state, comunicazione con backend via `invoke` |
| IPC | Tauri v2 commands | Bridge frontend↔backend, type-safe |
| Backend | Rust async (tokio) | Registrazione, download, gestione server STT |
| Audio capture | cpal | Microfono cross-platform |
| System audio | ScreenCaptureKit (macOS) | Cattura audio di sistema |
| OS permissions | CoreGraphics FFI + `open` (macOS) | `permissions.rs` — stato/richiesta permesso registrazione schermo, CTA per aprire i pannelli Privacy & Security (vedi `docs/backend/commands.md` → Permessi OS) |
| STT | whisper.cpp server | Trascrizione locale, API OpenAI-compatible |
| File I/O | hound | Scrittura WAV |

## Stato persistente

- `RecorderState` — `tokio::sync::Mutex<RecorderInner>`, managed da Tauri, vive per tutta la sessione app
- `settings.json` — `SttSettings` serializzato in `app_data_dir/settings.json`
- `app_data_dir/bin/whisper-server` — binary scaricato una volta
- `app_data_dir/models/ggml-large-v3-turbo.bin` — modello (~1.5 GB), scaricato una volta

## Dipendenze chiave

| Crate/lib | Versione | Uso |
|---|---|---|
| tauri | 2.x | Framework desktop |
| cpal | 0.17 | Audio I/O cross-platform |
| screencapturekit | 7.x | System audio macOS |
| hound | 3.x | WAV encoding |
| reqwest | 0.12 | HTTP client per whisper API |
| tokio | 1.x | Async runtime |
| zip | 2.x | Estrazione binary whisper |

# Architettura

## Struttura repo

`src/` (frontend Svelte) + `src-tauri/` (backend Rust) a top-level: layout standard generato da `npm create tauri-app` / template ufficiali Tauri+Vite. `src-tauri/` deve restare a root perché Cargo.toml e config bundler usano path relativi fissi rispetto alla root del progetto. Layout monorepo (`apps/frontend` + `apps/backend`) avrebbe senso solo con più target/build separate da condividere — non è il caso qui (single-app desktop).

## Flusso dati end-to-end

```
[Microfono]         [Audio sistema]
    │                     │
    │ cpal input           │ ScreenCaptureKit (macOS) — cattura già a
    │ rate nativo device   │ TARGET_SAMPLE_RATE (16kHz, conversione nativa OS)
    │ (mic_native_rate)    │ PipeWire monitor (Linux)
    ▼                     ▼
[mic_samples: Arc<Mutex<Vec<f32>>>]   [sys_samples: Arc<Mutex<Vec<f32>>>]
                │                              │
                ▼                              │
       [resample_linear()]                    │   (solo mic: cpal non forza un
       mic_native_rate → TARGET_SAMPLE_RATE   │    rate di cattura arbitrario;
       (interpolazione lineare, condivisa     │    sys arriva già al rate giusto)
       con prepare_for_whisper)               │
                │                              │
                ├──────────────┬───────────────┤   (buffer ora allo stesso rate —
                │              │               │    usati due volte prima di essere fusi)
                ▼              │               ▼
       [audio::mix()]         │      [diarization::estimate_timeline()]
       somma sample-by-sample │      energia RMS mic vs sistema a finestre
       clamp ±1.0             │      → intervalli {start, end, label}
                │             │               │
                ▼             │               ▼
       [audio::write_wav()] │      [<nome>.diarization.json]
       WAV Int16 PCM         │      sidecar accanto al WAV
       16kHz fisso/channels  │      (solo se audio sistema attivo,
                │            │       best-effort — non blocca la REC)
                ▼            │               │
       [file .wav su disco] ─┘               │
                │                            │
                ▼                            │
   [transcribe_recording() command]          │
   multipart POST /inference                 │
   language: it, response_format:            │
   verbose_json (prepare_for_whisper         │
   converte in memoria — passthrough per     │
   nuove REC già a 16kHz, resample solo per  │
   file legacy/stereo)                       │
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

### Entry point alternativo: import file esterno

`import_audio_file` è una seconda sorgente per lo stesso pipeline. Invece di
mic+sistema, decodifica un file audio scelto dall'utente (symphonia: wav/mp3/
m4a/mp4/flac/ogg) in campioni f32, li porta a mono 16kHz (downmix + lo stesso
`resample_linear` del ramo mic) e scrive `recording.wav` in una nuova cartella
Records — da lì il flusso è identico a una registrazione (`transcribe_recording`
→ whisper-server → `transcript.json`). Nessun ramo diarizzazione: sorgente
singola, `speaker = None`.

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
| symphonia | 0.5 | Decode file audio importati (mp3/m4a/mp4/flac/ogg) — Rust puro, no ffmpeg |
| reqwest | 0.12 | HTTP client per whisper API |
| tokio | 1.x | Async runtime |
| zip | 2.x | Estrazione binary whisper |

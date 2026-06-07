# Architettura

## Flusso dati end-to-end

```
[Microfono]         [Audio sistema]
    │                     │
    │ cpal input           │ ScreenCaptureKit (macOS)
    │                     │ PipeWire monitor (Linux)
    ▼                     ▼
[mic_samples: Arc<Mutex<Vec<f32>>>]   [sys_samples: Arc<Mutex<Vec<f32>>>]
                │                              │
                └──────────┬───────────────────┘
                           │
                    [mixer::mix()]
                    somma sample-by-sample
                    clamp ±1.0
                           │
                           ▼
                    [writer::write_wav()]
                    WAV Float32, sample_rate/channels
                    da dialog save utente
                           │
                           ▼
                    [file .wav su disco]
                           │
                           ▼
              [transcribe_recording() command]
              multipart POST /v1/audio/transcriptions
              language: it, response_format: verbose_json
                           │
                    [whisper-server locale]
                    127.0.0.1:8080
                    modello: ggml-large-v3-turbo
                           │
                           ▼
                    [TranscriptResult]
                    text, segments[], speaker diarization
                           │
                           ▼
                    [Frontend React]
                    TranscriptView — gruppi per speaker
```

## Componenti principali

| Layer | Tecnologia | Ruolo |
|---|---|---|
| Frontend | React 19 + TypeScript | UI, state, comunicazione con backend via `invoke` |
| IPC | Tauri v2 commands | Bridge frontend↔backend, type-safe |
| Backend | Rust async (tokio) | Registrazione, download, gestione server STT |
| Audio capture | cpal | Microfono cross-platform |
| System audio | ScreenCaptureKit (macOS) | Cattura audio di sistema |
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

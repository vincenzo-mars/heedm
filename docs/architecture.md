# Architettura

Documento unico sul backend: struttura, flusso dati, pipeline audio e integrazione whisper.
Per la superficie API (comandi Tauri, tipi, componenti Svelte) vedi [`reference.md`](reference.md).

## Struttura repo

`src/` (frontend Svelte) + `src-tauri/` (backend Rust) a top-level: layout standard dei template Tauri+Vite. `src-tauri/` deve restare a root perché `Cargo.toml` e la config del bundler usano path relativi fissi rispetto alla root del progetto.

## Flusso dati end-to-end

```
[Microfono]                    [Audio sistema]
    │                               │
    │ cpal, rate e canali           │ ScreenCaptureKit (macOS), già a 16kHz
    │ nativi del device             │ PipeWire/PulseAudio monitor (Linux)
    ▼                               ▼
[mic_samples: Arc<Mutex<Vec<f32>>>]  [sys_samples: Arc<Mutex<Vec<f32>>>]
    │                               │
    ▼                               │
[audio::to_mono()]                  │  downmix esplicito: resample_linear su
    │                               │  buffer interleaved mescolerebbe i canali
    ▼                               │
[audio::resample_linear()]          │  solo il mic: cpal non forza un rate,
mic_native_rate → 16kHz             │  sys arriva già a TARGET_SAMPLE_RATE
    │                               │
    └──────────┬────────────────────┘
               ▼
        [aec::cancel_echo()]           solo se sys è attivo e non vuoto
        toglie dal mic l'audio delle
        casse rientrato dal microfono
               │
               ▼
        [audio::interleave_stereo()]   mic → canale L, sistema → canale R
               │                       (senza audio di sistema resta mono)
               ▼
        [audio::write_wav()]           WAV Int16 PCM, 16kHz
               │
               ▼
        [<timestamp>/recording.wav]
               │
               ▼
   [transcribe_recording()]            multipart POST /inference
   language=it, response_format=       i byte del file vanno così come sono:
   verbose_json, diarize=true          sono già nel formato di whisper.cpp
               │
               ▼
   [whisper-server locale]             127.0.0.1:8080
   ggml-large-v3-turbo                 diarizza confrontando L vs R
               │
               ▼
   [TranscriptResult]                  segments[].speaker = "0" | "1" | None
               │                       ("?" ambiguo → normalizzato a None)
               ▼
   [<timestamp>/transcript.json]
               │
               ▼
        [Frontend Svelte]              "0" → YOU, "1" → THEM
```

### Entry point alternativo: import di un file esterno

`import_audio_file` è una seconda sorgente per lo stesso pipeline: decodifica un file scelto dall'utente (symphonia: wav/mp3/m4a/mp4/flac/ogg/aac), lo porta a **mono** 16kHz e scrive `recording.wav` in una nuova cartella Records. Da lì il flusso è identico a una registrazione.

Il downmix a mono è voluto e va tenuto: i canali L/R di un file esterno (un export Zoom, un mp3) non sono "io" e "gli altri", sono solo stereo. Lasciarli separati farebbe produrre a whisper etichette speaker casuali. Essendo mono, `diarize` viene ignorato dal server e `speaker` resta `None`.

## Registrazione

### Stato (`recorder/mod.rs`)

`RecorderInner` è protetto da `tokio::sync::Mutex` e vive per tutta la sessione app.

| Campo | Tipo | Ruolo |
|---|---|---|
| `is_recording` | `bool` | Guardia contro doppio start/stop |
| `start_time` | `Option<Instant>` | Base per `duration_ms` |
| `mic_samples` / `sys_samples` | `Arc<Mutex<Vec<f32>>>` | Buffer di cattura, scritti dai callback audio |
| `mic_stream` | `Option<cpal::Stream>` | Va tenuto vivo: dropparlo ferma la cattura |
| `sys_capture` | `Option<Box<dyn SysAudioStop>>` | Handle della cattura di sistema |
| `sample_rate` | `u32` | Rate di **output**, fisso a `TARGET_SAMPLE_RATE` |
| `mic_native_rate` | `u32` | Rate nativo del device, dipende dall'hardware |
| `channels` | `u16` | Canali del microfono; serve solo a `to_mono` |

### Microfono (`recorder/mic.rs`)

`cpal` sul default input device, con la sua `default_input_config()`: rate e canali non sono forzati, si prende quello che il device espone. Supporta i formati `F32`, `I16` e `U16` normalizzando tutto a `f32` in `[-1.0, 1.0]`.

### Audio di sistema (`recorder/system_audio/`)

- **macOS** (`macos.rs`): `SCStream` di ScreenCaptureKit sul display primario. La configurazione chiede direttamente `TARGET_SAMPLE_RATE`, che il framework converte nativamente: nessun resampling lato nostro. Richiede il permesso Screen Recording (`NSScreenCaptureUsageDescription` in `Info.plist`).
- **Linux** (`linux.rs`): `cpal` sul primo input device il cui nome contiene "monitor" (PulseAudio/PipeWire).
- Altre piattaforme: `system_audio::start` ritorna un errore esplicito. Windows non è supportato.

Se la cattura di sistema fallisce la registrazione **non** si interrompe: l'errore viene loggato e si registra il solo microfono.

### Primitive condivise (`recorder/audio.rs`)

`TARGET_SAMPLE_RATE` (16000, il rate che `read_wav` di whisper.cpp richiede in `common.cpp`) più le funzioni usate da più moduli:

| Funzione | Note |
|---|---|
| `rms(&[f32]) -> f32` | Energia; usata da AEC |
| `to_mono(Vec<f32>, channels) -> Vec<f32>` | Prende ownership per restituire il buffer intatto quando è già mono. Va sempre chiamata **prima** di `resample_linear` |
| `resample_linear(&[f32], from, to)` | Interpolazione lineare pura-Rust, nessuna dipendenza esterna. Assume input **mono** |
| `interleave_stereo(left, right)` | Intreccia mic e sistema zero-paddando il più corto |
| `write_wav(&[f32], path, rate, channels)` | `hound`, Int16 PCM |

### Echo cancellation (`recorder/aec.rs`)

Serve a registrare con gli altoparlanti accesi invece che in cuffia. Stima il ritardo fra `sys` e la sua eco nel microfono via cross-correlazione (finestra di 2s, ritardo massimo 200ms), poi sottrae la componente stimata con un coefficiente ai minimi quadrati. Se la correlazione massima è sotto `MIN_CORR` (0.10) l'eco è trascurabile e il mic torna invariato.

Non è solo qualità audio: **la diarizzazione dipende da questo**. L'audio di sistema rientrato nel canale mic falserebbe il confronto di energia con cui whisper attribuisce gli speaker.

Limiti: copre il percorso di eco diretto dominante, non il riverbero multi-path, e assume un ritardo stabile per tutta la registrazione.

## STT

### Binario e modello

`whisper-server` (whisper.cpp) è compilato da `scripts/build-whisper-server.sh` come binario universale macOS (arm64 + x86_64), statico e con Metal attivo, e finisce in `src-tauri/binaries/`. Tauri lo bundla come risorsa (`tauri.conf.json` → `bundle.resources`) e a runtime `bundled_bin_path` lo risolve con `app.path().resolve(..., BaseDirectory::Resource)`.

Il modello **non** è bundled: `download_local_model` scarica `ggml-large-v3-turbo.bin` (~1.5 GB) da Hugging Face in streaming, emettendo eventi `download-progress` con payload `{ step: "model" | "done", pct }`, e a fine download imposta `local_ready` nelle settings.

### Ciclo di vita del server

`WhisperServerState` tiene l'unico handle al processo. `start_local_server`:

1. Se la porta 8080 è già in ascolto, esce subito
2. Verifica che il modello esista su disco
3. Spawna il binario con `--model/--host/--port`, più `--flash-attn` e `--threads`
4. Polla la porta ogni secondo fino a 60s

Flash attention e numero di thread non hanno UI: si deducono dalla macchina (`available_parallelism` meno due, per lasciare respiro al resto del sistema). `--flash-attn` esiste solo dalle build recenti di whisper.cpp, quindi il tag pinnato in `scripts/build-whisper-server.sh` e i flag passati qui devono restare allineati: passare `-fa` a un binario vecchio lo fa uscire con un errore di parsing.

In `lib.rs` l'app è costruita con `.build()` invece di `.run()` diretto, così il loop eventi può intercettare `RunEvent::ExitRequested` e terminare il figlio, altrimenti `whisper-server` resterebbe in background dopo il quit.

### Trascrizione

`transcribe_recording` legge il WAV e lo invia in multipart a `POST /inference` con `language=it`, `response_format=verbose_json` e `diarize=true`. Il file **non** viene convertito: quello che Heedm produce è già mono/stereo 16kHz 16-bit PCM, cioè esattamente ciò che `read_wav` accetta.

Il risultato viene arricchito con `transcription_ms` (misurato lato client, non presente nella risposta del server) e scritto come `transcript.json` accanto al WAV.

### Diarizzazione: la fa il server

Con `diarize=true` e un file **stereo**, whisper.cpp confronta l'energia dei due canali su ogni segmento (`estimate_diarization_speaker` in `server.cpp`, soglia 1.1) e mette il risultato nel campo `speaker` di ogni segmento del `verbose_json`:

- `"0"` → canale sinistro → microfono → **YOU**
- `"1"` → canale destro → audio di sistema → **THEM**
- `"?"` → energie a meno del 10% l'una dall'altra

`normalize_speaker` riduce l'ambiguità a `None`, lo stesso stato dei file mono: le etichette sono due, non tre. Sui file mono il server ignora `diarize` da solo (`pcmf32s.size() == 2` è falso), quindi non serve condizionare il campo lato client.

Verificato empiricamente: due voci distinte su canali separati escono correttamente come `speaker` `"0"` e `"1"`; senza il campo `diarize` il campo `speaker` non compare affatto.

## Stato persistente

- `settings.json` — `SttSettings` serializzato in `app_data_dir/`
- `app_data_dir/models/ggml-large-v3-turbo.bin` — il modello
- `~/Documents/Heedm/Records/<timestamp>/` — `recording.wav` + `transcript.json`

## Dipendenze chiave

| Crate/lib | Versione | Uso |
|---|---|---|
| tauri | 2.x | Framework desktop |
| cpal | 0.17 | Audio input cross-platform |
| screencapturekit | 7.x | System audio macOS |
| hound | 3.x | Encoding WAV |
| symphonia | 0.5 | Decodifica dei file importati, Rust puro, niente ffmpeg |
| reqwest | 0.12 | HTTP verso whisper-server e Hugging Face |
| tokio | 1.x | Runtime async |

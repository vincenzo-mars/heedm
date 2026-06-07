# STT — integrazione Whisper locale

File: `src-tauri/src/commands.rs` (sezione STT)

## Architettura

Heedm usa **whisper.cpp** come server HTTP locale. Non ci sono chiamate cloud.

```
app → whisper-server (127.0.0.1:8080) → modello ggml → TranscriptResult
```

## Configurazione

| Costante | Valore |
|---|---|
| `LOCAL_PORT` | `8080` |
| Binary | `whisper-server` — bundled nell'app, vedi sezione dedicata sotto |
| Modello | `ggml-large-v3-turbo.bin` da HuggingFace (ggerganov/whisper.cpp) |
| Lingua | `it` (hardcoded in `transcribe_recording`) |

## Percorsi su disco

| File | Percorso default | Configurabile |
|---|---|---|
| Binary | risorsa bundled dell'app (`resource_dir`/`whisper-server`) | no — incluso nell'app, nessun download |
| Modello | `{model_dir}/models/ggml-large-v3-turbo.bin` | sì, via `settings.model_dir` |
| Settings | `app_data_dir/settings.json` | no |
| Registrazioni WAV | `{recordings_dir}/Registrazione <YYYY-MM-DD HH.MM.SS>.wav` | sì, via `settings.recordings_dir` |

Default quando l'utente non ha scelto un percorso (`model_dir`/`recordings_dir` = `null`):
- `model_dir` → `app_data_dir` (macOS: `~/Library/Application Support/com.vincenzomars.heedm/`) — dati app persistenti, non visibili in Finder normale
- `recordings_dir` → `audio_dir()/Heedm/Records` (macOS: `~/Music/Heedm/Records/`) — cartella standard contenuti audio, visibile e indicizzata da Spotlight; sottocartella `Records` separa le registrazioni da eventuali altri file dell'app dentro `Heedm`

Nome file generato con timestamp leggibile locale (`chrono::Local::now()`, formato `%Y-%m-%d %H.%M.%S`), es. `Registrazione 2026-06-07 15.30.12.wav`.

Funzioni helper in `commands.rs`: `model_dir`, `bundled_bin_path`, `local_model_path`, `default_recordings_dir`, `recordings_dir` — tutte (tranne `bundled_bin_path`, che non dipende dalle settings) prendono `&SttSettings` già caricato per evitare letture multiple di `settings.json`.

**Nota cambio `model_dir`:** non c'è migrazione automatica del modello (~1.5GB). Cambiando cartella, l'app non lo trova più nel nuovo percorso e richiede un nuovo download lì. Il pannello impostazioni avvisa l'utente prima del cambio. Il binario non è interessato — è bundled con l'app, non dipende da `model_dir`.

## Binario `whisper-server` — bundled come risorsa app

whisper.cpp **non distribuisce** un binario `server` precompilato per macOS via GitHub Releases (verificato: nessun asset `osx`/`macos`/`darwin`/`server` in nessuna release). L'app quindi **include il binario compilato** anziché scaricarlo a runtime — scelta dettata dal vincolo "app usabile da utenti finali" senza richiedere Homebrew o toolchain di compilazione (Xcode CLI Tools/cmake) sulla macchina dell'utente. Decisione e alternative scartate documentate in `DEVLOG.md`.

Meccanismo (Tauri v2 `bundle.resources`):
- `tauri.conf.json` → `bundle.resources: { "binaries/whisper-server": "whisper-server" }` — il file in `src-tauri/binaries/whisper-server` viene copiato come risorsa dell'app (sia in dev che nel bundle finale, via `tauri_build::build()` chiamato da `build.rs`)
- A runtime, `bundled_bin_path` (`commands.rs`) risolve il percorso con `app.path().resolve("whisper-server", BaseDirectory::Resource)`

Build del binario — script `scripts/build-whisper-server.sh`:
- Compila whisper.cpp (`examples/server`, target cmake `whisper-server`) con `BUILD_SHARED_LIBS=OFF` (binario statico, zero dylib esterne da bundlare — verificato via `otool -L`) e `GGML_METAL=ON` (accelerazione GPU su Apple Silicon e Intel)
- Compila per `arm64` e `x86_64` separatamente (`CMAKE_OSX_ARCHITECTURES`) e unisce con `lipo -create` in un **binario universale**, coerente con `minimumSystemVersion: 13.0` (entrambe le architetture supportate)
- Output in `src-tauri/binaries/whisper-server` (gitignored — binario grande e generato, non si committa)

`src-tauri/binaries/` deve esistere con il binario prima di `cargo build`/`tauri dev` — altrimenti Tauri fallisce con `resource path 'binaries/whisper-server' doesn't exist`. Aggiornare whisper.cpp = ri-lanciare lo script puntato a un tag più recente e ricompilare l'app.

## Strutture dati

### `SttSettings`
```rust
pub struct SttSettings {
    pub local_ready: bool,           // binario + modello presenti su disco
    pub configured: bool,            // utente ha completato setup
    pub model_dir: Option<String>,   // None = default (app_data_dir)
    pub recordings_dir: Option<String>, // None = default (audio_dir/Heedm)
}
```
Serializzata come JSON in `settings.json` (camelCase).

### `TranscriptResult`
```rust
pub struct TranscriptResult {
    pub task: String,
    pub language: String,
    pub duration: f64,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}
```

### `TranscriptSegment`
```rust
pub struct TranscriptSegment {
    pub id: u32,
    pub start: f64,    // secondi
    pub end: f64,
    pub text: String,
    pub speaker: Option<String>,  // es. "SPEAKER_00"
}
```

## Comandi percorso

| Comando | Ritorna | Uso |
|---|---|---|
| `get_local_model_path` | path assoluto del file modello | mostrato in UI; crea sempre la cartella padre (`models/`) se assente, così "Mostra nel Finder" funziona anche prima del download (rivela la cartella, non il file — che potrebbe non esistere ancora) |
| `get_recordings_dir` | path assoluto cartella registrazioni corrente | mostrato in UI + "Mostra nel Finder" |
| `pick_directory` | `Option<String>` (cartella scelta o `None` se annullato) | dialog `pick_folder`, riusato sia per modello che registrazioni |

## Flusso download (`download_local_model`)

Solo il modello viene scaricato — il binario `whisper-server` è già incluso nell'app (vedi sezione sopra), nessun download necessario:

1. Scarica modello `.bin` da HuggingFace con progress events (`"model"` step)
2. Emette evento `download-progress { step: "done", pct: 100 }`
3. Aggiorna `settings.local_ready = true` e salva

Gli eventi sono `"download-progress"` con payload `{ step: "model"|"done", pct: 0-100 }`.

## Avvio server (`start_local_server`)

1. Verifica se porta 8080 è già in ascolto → se sì, esce subito
2. Risolve il binario bundled (`bundled_bin_path`) e controlla che il modello esista
3. Spawna processo: `whisper-server --model <path> --host 127.0.0.1 --port 8080`
4. Polling TCP ogni 1s per 60s max → `Err` se non risponde

## Trascrizione (`transcribe_recording`)

POST `http://127.0.0.1:8080/inference` con multipart (whisper.cpp espone solo `/inference`, `/load`, `/health` — **non** un endpoint OpenAI-compatible `/v1/audio/transcriptions`):
- `file`: bytes WAV **convertiti** (vedi sotto)
- `language`: `"it"`
- `response_format`: `"verbose_json"`

Risponde con `TranscriptResult` JSON.

### Conversione audio in memoria (`prepare_for_whisper`)

`whisper.cpp`'s `read_wav` (`common.cpp`) accetta **solo** WAV mono, 16kHz, 16-bit PCM — qualsiasi altro formato → `{"error":"failed to read WAV file"}`. Le registrazioni sono salvate a piena qualità (rate nativo del microfono via cpal — dinamico, dipende dall'hardware, es. 44100/48000 Hz —, float32, vedi `recorder/writer.rs`), per preservare l'archivio dell'utente.

`prepare_for_whisper` (`commands.rs`) converte **solo in memoria**, al momento dell'invio, senza toccare il file salvato:
1. Legge il WAV con `hound::WavReader`, normalizza i sample a `f32` (gestisce sia `Float` che `Int`)
2. Downmix a mono (media dei canali, se stereo)
3. Resample al rate target (16kHz) via interpolazione lineare — rate arbitrario in ingresso, non un rapporto fisso (44100→16000 non è un intero)
4. Converte `f32` → `i16` (clamp + scala per `i16::MAX`)
5. Ri-codifica come WAV 16-bit PCM mono in un buffer `Cursor<Vec<u8>>` via `hound::WavWriter`, inviato come bytes multipart

Eseguita in `tokio::task::spawn_blocking` (CPU-bound, non deve bloccare il runtime async). Nessuna dipendenza esterna (niente `ffmpeg`/`--convert` lato server, niente crate di resampling — interpolazione lineare è sufficiente per parlato/ASR e coerente col principio "no toolchain esterni" già adottato per il bundling del binario).

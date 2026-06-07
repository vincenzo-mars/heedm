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
| Binary (arm64) | `whisper-blas-blas-server-osx-arm64.zip` da whisper.cpp v1.7.4 |
| Binary (x64) | `whisper-blas-blas-server-osx-x64.zip` da whisper.cpp v1.7.4 |
| Modello | `ggml-large-v3-turbo.bin` da HuggingFace (ggerganov/whisper.cpp) |
| Lingua | `it` (hardcoded in `transcribe_recording`) |

## Percorsi su disco

| File | Percorso default | Configurabile |
|---|---|---|
| Binary | `{model_dir}/bin/whisper-server` | sì, via `settings.model_dir` |
| Modello | `{model_dir}/models/ggml-large-v3-turbo.bin` | sì, via `settings.model_dir` |
| Settings | `app_data_dir/settings.json` | no |
| Registrazioni WAV | `{recordings_dir}/Registrazione <YYYY-MM-DD HH.MM.SS>.wav` | sì, via `settings.recordings_dir` |

Default quando l'utente non ha scelto un percorso (`model_dir`/`recordings_dir` = `null`):
- `model_dir` → `app_data_dir` (macOS: `~/Library/Application Support/com.vincenzomars.heedm/`) — dati app persistenti, non visibili in Finder normale
- `recordings_dir` → `audio_dir()/Heedm/Records` (macOS: `~/Music/Heedm/Records/`) — cartella standard contenuti audio, visibile e indicizzata da Spotlight; sottocartella `Records` separa le registrazioni da eventuali altri file dell'app dentro `Heedm`

Nome file generato con timestamp leggibile locale (`chrono::Local::now()`, formato `%Y-%m-%d %H.%M.%S`), es. `Registrazione 2026-06-07 15.30.12.wav`.

Funzioni helper in `commands.rs`: `model_dir`, `local_bin_path`, `local_model_path`, `default_recordings_dir`, `recordings_dir` — tutte prendono `&SttSettings` già caricato per evitare letture multiple di `settings.json`.

**Nota cambio `model_dir`:** non c'è migrazione automatica dei file (binary + modello ~1.5GB). Cambiando cartella, l'app non trova più binary/modello nel nuovo percorso e richiede un nuovo download lì. Il pannello impostazioni avvisa l'utente prima del cambio.

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

1. Scarica zip binary da GitHub Releases con progress events (`"binary"` step)
2. Estrae entry `server` o `*/server` dallo zip, scrive in `bin/whisper-server`, chmod 755
3. Scarica modello `.bin` da HuggingFace con progress events (`"model"` step)
4. Emette evento `download-progress { step: "done", pct: 100 }`
5. Aggiorna `settings.local_ready = true` e salva

Gli eventi sono `"download-progress"` con payload `{ step: "binary"|"model"|"done", pct: 0-100 }`.

## Avvio server (`start_local_server`)

1. Verifica se porta 8080 è già in ascolto → se sì, esce subito
2. Controlla che binario e modello esistano
3. Spawna processo: `whisper-server --model <path> --host 127.0.0.1 --port 8080`
4. Polling TCP ogni 1s per 60s max → `Err` se non risponde

## Trascrizione (`transcribe_recording`)

POST `http://127.0.0.1:8080/v1/audio/transcriptions` con multipart:
- `file`: bytes del WAV con filename originale
- `model`: `"whisper-1"` (campo richiesto dall'API, ignorato da whisper.cpp)
- `language`: `"it"`
- `response_format`: `"verbose_json"`

Risponde con `TranscriptResult` JSON.

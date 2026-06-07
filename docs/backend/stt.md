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

Tutti relativi a `app_data_dir` (macOS: `~/Library/Application Support/com.vincenzomars.heedm/`):

| File | Percorso |
|---|---|
| Binary | `bin/whisper-server` |
| Modello | `models/ggml-large-v3-turbo.bin` |
| Settings | `settings.json` |

## Strutture dati

### `SttSettings`
```rust
pub struct SttSettings {
    pub local_ready: bool,   // binario + modello presenti su disco
    pub configured: bool,    // utente ha completato setup
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

# Recorder — pipeline di registrazione audio

File: `src-tauri/src/recorder/`

## Structs condivise (`mod.rs`)

### `SysAudioStop`
Trait per fermare la cattura audio di sistema. Ogni implementazione platform-specific lo implementa.
```rust
pub trait SysAudioStop: Send {
    fn stop(&mut self) -> Result<(), String>;
}
```

### `RecorderInner`
Stato interno della registrazione. Tenuto sotto `tokio::sync::Mutex`.

| Campo | Tipo | Descrizione |
|---|---|---|
| `is_recording` | `bool` | Flag attivo durante la registrazione |
| `start_time` | `Option<Instant>` | Per calcolo durata |
| `mic_samples` | `Arc<Mutex<Vec<f32>>>` | Buffer campioni microfono |
| `sys_samples` | `Arc<Mutex<Vec<f32>>>` | Buffer campioni audio sistema |
| `mic_stream` | `Option<cpal::Stream>` | Stream cpal attivo |
| `sys_capture` | `Option<Box<dyn SysAudioStop>>` | Handle cattura sistema |
| `sample_rate` | `u32` | Rate di **output** della registrazione — fisso, = `TARGET_SAMPLE_RATE` (16000, lo stesso richiesto da whisper). Usato da mixer/diarizzazione/writer |
| `mic_native_rate` | `u32` | Rate **nativo** del device microfono al momento della cattura (cpal, dinamico, dipende dall'hardware). Serve solo per ricampionare il buffer mic verso `sample_rate` a fine registrazione |
| `channels` | `u16` | Canali del microfono (default 2) |

### `RecorderState`
Wrapper Tauri-managed: `pub struct RecorderState(pub tokio::sync::Mutex<RecorderInner>)`.

---

## Microfono (`mic.rs`)

### `start_mic_capture(buf: Arc<Mutex<Vec<f32>>>) -> Result<MicInfo, String>`
- Usa `cpal::default_host()` e il device di input default
- Supporta formati: `F32` (pass-through), `I16` (normalizza ÷ i16::MAX), `U16` (normalizza, centra su 0)
- Accumula campioni in `buf` via callback stream
- Restituisce `MicInfo { stream, sample_rate, channels }`

Il `sample_rate` restituito è quello **nativo** del device (cpal non garantisce
di poter forzare un rate arbitrario in cattura) — salvato in `mic_native_rate` e
ricampionato a `TARGET_SAMPLE_RATE` in `stop_recording` (vedi sezione Mixer).

Il `Stream` va tenuto vivo in `RecorderInner::mic_stream` — dropparlo ferma la cattura.

---

## Mixer (`mixer.rs`)

### `mix(mic: &[f32], sys: &[f32]) -> Vec<f32>`
Somma i due buffer sample-by-sample fino al più lungo (il più corto viene
considerato silenzio/zero-padded oltre la propria lunghezza).
Clamp a `[-1.0, 1.0]` per prevenire clipping.

Richiede che `mic`/`sys` siano già allo **stesso rate** (`sample_rate`,
`TARGET_SAMPLE_RATE`): `stop_recording` (`commands.rs`) ricampiona il buffer mic
dal suo rate nativo (`mic_native_rate`) a `sample_rate` con `resample_linear`
(interpolazione lineare pura-Rust, stessa logica condivisa con
`prepare_for_whisper`, vedi [`docs/backend/stt.md`](stt.md)) **prima** di
chiamare `mix`/`estimate_timeline`. L'audio di sistema arriva già al rate
giusto — su macOS, `SCStream` lo cattura nativamente a `TARGET_SAMPLE_RATE`
(nessun resample necessario lato nostro).

---

## Diarizzazione (`diarization.rs`)

### `estimate_timeline(mic: &[f32], sys: &[f32], sample_rate: u32) -> Vec<SpeakerInterval>`
Stima — *prima* del mix in mono — chi sta parlando confrontando l'energia RMS di mic
e audio di sistema a finestre fisse (200ms), poi accorpa finestre consecutive con la
stessa etichetta in intervalli `{ start, end, label }` (formato compatto da
serializzare).

`SpeakerLabel`: `Mic` | `Sys` | `Both` (energie comparabili — sovrapposizione).
Soglia: `DOMINANCE_RATIO = 1.5`.

Modello "a 2 vie" (tu = mic vs. audio di sistema) — è il massimo che la pipeline può
distinguere: mic e sistema sono due sorgenti separate, ma ciascuna può comunque
contenere più persone (es. una chiamata multi-partecipante lato sistema viene
etichettata come un unico "sistema").

### Sidecar `recording.diarization.json`
`stop_recording` (in `commands.rs`) chiama `estimate_timeline` sui buffer grezzi
**prima** di passarli a `mixer::mix` — una volta fusi in un'unica traccia mono
l'informazione "chi" è persa — e scrive il risultato come JSON nella cartella
della registrazione come `recording.diarization.json` (stessa cartella di `recording.wav`).

Scritto solo se la cattura audio di sistema era attiva (`sys_capture.is_some()`):
senza una seconda sorgente il timeline sarebbe banalmente sempre "mic". È
best-effort — un fallimento di scrittura del sidecar viene loggato e ignorato,
non fa fallire la registrazione (il WAV è già su disco a quel punto).

`transcribe_recording` (vedi [`docs/backend/stt.md`](stt.md)) carica questo sidecar
se presente e lo usa per popolare `TranscriptSegment.speaker`.

### Struttura cartella per registrazione

```
Records/
  └── YYYY-MM-DD HH.MM.SS/
       ├── recording.wav
       ├── recording.diarization.json   (solo se audio di sistema attivo)
       └── transcript.json              (scritto da transcribe_recording, best-effort)
```

---

## Writer (`writer.rs`)

### `write_wav(samples: &[f32], path: &Path, sample_rate: u32, channels: u16) -> Result<(), String>`
Scrive un file WAV in formato `Int16 PCM` usando `hound::WavWriter` (converte ogni sample `f32` con clamp `[-1.0, 1.0]` poi scala per `i16::MAX`).
Spec WAV output: `{ bits_per_sample: 16, sample_format: Int, channels, sample_rate }`.

---

## Audio di sistema

### macOS (`system_audio/macos.rs`)
- Usa `ScreenCaptureKit` via crate `screencapturekit`
- Crea uno `SCStream` sul display primario con `SCContentFilter::display_excluding_windows([])`
- Configura stream: `sample_rate` = `TARGET_SAMPLE_RATE` (16kHz fisso, passato da `start_recording` — `SCStream` lo converte nativamente, zero resampling lato nostro), `channels` ereditato dal microfono, `excludes_current_process_audio: false`
- `AudioHandler` implementa `SCStreamOutputTrait`: estrae campioni F32 da `CMSampleBuffer`, accumula in buffer
- `stop()` chiama `SCStream::stop_capture()`

Richiede permesso `NSScreenCaptureUsageDescription` (in `Info.plist`).

### Linux (`system_audio/linux.rs`)
- Usa `cpal` con device "monitor" (PulseAudio/PipeWire)
- Cerca device con nome contenente "monitor" tra gli input devices
- Supporta F32 e I16 (stessa normalizzazione di mic.rs)

### Windows (`system_audio/windows.rs`)
Stub — ritorna `Err("Windows system audio capture not yet implemented")`.

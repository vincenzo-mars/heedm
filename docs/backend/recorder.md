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
| `sample_rate` | `u32` | Sample rate del microfono (default 48000) |
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

Il `Stream` va tenuto vivo in `RecorderInner::mic_stream` — dropparlo ferma la cattura.

---

## Mixer (`mixer.rs`)

### `mix(mic: &[f32], sys: &[f32]) -> Vec<f32>`
Somma i due buffer sample-by-sample. Se hanno lunghezze diverse, si ferma al più corto.
Clamp a `[-1.0, 1.0]` per prevenire clipping.

---

## Writer (`writer.rs`)

### `write_wav(samples: &[f32], path: &Path, sample_rate: u32, channels: u16) -> Result<(), String>`
Scrive un file WAV in formato `Float32` usando `hound::WavWriter`.
Spec WAV output: `{ bits_per_sample: 32, sample_format: Float, channels, sample_rate }`.

---

## Audio di sistema

### macOS (`system_audio/macos.rs`)
- Usa `ScreenCaptureKit` via crate `screencapturekit`
- Crea uno `SCStream` sul display primario con `SCContentFilter::display_excluding_windows([])`
- Configura stream: `sample_rate` e `channels` ereditati dal microfono, `excludes_current_process_audio: false`
- `AudioHandler` implementa `SCStreamOutputTrait`: estrae campioni F32 da `CMSampleBuffer`, accumula in buffer
- `stop()` chiama `SCStream::stop_capture()`

Richiede permesso `NSScreenCaptureUsageDescription` (in `Info.plist`).

### Linux (`system_audio/linux.rs`)
- Usa `cpal` con device "monitor" (PulseAudio/PipeWire)
- Cerca device con nome contenente "monitor" tra gli input devices
- Supporta F32 e I16 (stessa normalizzazione di mic.rs)

### Windows (`system_audio/windows.rs`)
Stub — ritorna `Err("Windows system audio capture not yet implemented")`.

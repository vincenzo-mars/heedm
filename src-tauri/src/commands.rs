use std::io::Cursor;
use std::path::PathBuf;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::io::AsyncWriteExt;

use crate::permissions;
use crate::recorder::{diarization, mic, mixer, system_audio, writer, RecorderState};

// ── Constants ────────────────────────────────────────────────────────────────

const LOCAL_PORT: u16 = 8080;

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";

// ── Settings ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SttSettings {
    pub local_ready: bool,
    pub configured: bool,
    pub model_dir: Option<String>,
    pub recordings_dir: Option<String>,
}

impl Default for SttSettings {
    fn default() -> Self {
        Self {
            local_ready: false,
            configured: false,
            model_dir: None,
            recordings_dir: None,
        }
    }
}

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("heedm"))
}

fn settings_path(app: &AppHandle) -> PathBuf {
    app_data_dir(app).join("settings.json")
}

fn model_dir(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    match &settings.model_dir {
        Some(dir) => PathBuf::from(dir),
        None => app_data_dir(app),
    }
}

fn bundled_bin_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve("whisper-server", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())
}

fn local_model_path(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    model_dir(app, settings)
        .join("models")
        .join("ggml-large-v3-turbo.bin")
}

fn default_recordings_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .document_dir()
        .unwrap_or_else(|_| app_data_dir(app))
        .join("Heedm")
        .join("Records")
}

fn recordings_dir(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    match &settings.recordings_dir {
        Some(dir) => PathBuf::from(dir),
        None => default_recordings_dir(app),
    }
}

#[tauri::command]
pub async fn get_stt_settings(app: AppHandle) -> SttSettings {
    let path = settings_path(&app);
    tokio::fs::read_to_string(&path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn save_stt_settings(app: AppHandle, settings: SttSettings) -> Result<(), String> {
    let path = settings_path(&app);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_local_model_status(app: AppHandle) -> bool {
    let settings = get_stt_settings(app.clone()).await;
    local_model_path(&app, &settings).exists()
}

#[tauri::command]
pub async fn get_local_model_path(app: AppHandle) -> String {
    let settings = get_stt_settings(app.clone()).await;
    let path = local_model_path(&app, &settings);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    path.to_string_lossy().into_owned()
}

#[tauri::command]
pub async fn get_recordings_dir(app: AppHandle) -> String {
    let settings = get_stt_settings(app.clone()).await;
    let dir = recordings_dir(&app, &settings);
    tokio::fs::create_dir_all(&dir).await.ok();
    dir.to_string_lossy().into_owned()
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

// ── Download ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn download_local_model(app: AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    let settings = get_stt_settings(app.clone()).await;

    // whisper-server è bundled come risorsa app — qui si scarica solo il modello
    let model_path = local_model_path(&app, &settings);
    tokio::fs::create_dir_all(model_path.parent().unwrap())
        .await
        .map_err(|e| e.to_string())?;

    {
        let resp = client
            .get(MODEL_URL)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let total = resp.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(&model_path)
            .await
            .map_err(|e| e.to_string())?;
        let mut downloaded = 0u64;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                app.emit(
                    "download-progress",
                    serde_json::json!({"step":"model","pct": downloaded * 100 / total}),
                )
                .ok();
            }
        }
    }

    app.emit(
        "download-progress",
        serde_json::json!({"step":"done","pct":100}),
    )
    .ok();

    // Mark ready in settings
    let mut settings = get_stt_settings(app.clone()).await;
    settings.local_ready = true;
    save_stt_settings(app, settings).await
}

// ── Server management ─────────────────────────────────────────────────────────

/// Tiene l'handle del processo whisper-server per poterlo terminare
/// alla chiusura dell'app (altrimenti resta in background dopo il quit).
pub struct WhisperServerState(pub std::sync::Mutex<Option<tokio::process::Child>>);

impl WhisperServerState {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }
}

#[tauri::command]
pub async fn start_local_server(app: AppHandle) -> Result<(), String> {
    if tokio::net::TcpStream::connect(format!("127.0.0.1:{LOCAL_PORT}"))
        .await
        .is_ok()
    {
        return Ok(());
    }

    let settings = get_stt_settings(app.clone()).await;
    let bin = bundled_bin_path(&app)?;
    let model = local_model_path(&app, &settings);

    if !model.exists() {
        return Err("Modello locale non scaricato".to_string());
    }

    let child = tokio::process::Command::new(&bin)
        .args([
            "--model",
            model.to_str().unwrap(),
            "--host",
            "127.0.0.1",
            "--port",
            &LOCAL_PORT.to_string(),
        ])
        .spawn()
        .map_err(|e| e.to_string())?;

    *app.state::<WhisperServerState>().0.lock().unwrap() = Some(child);

    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{LOCAL_PORT}"))
            .await
            .is_ok()
        {
            return Ok(());
        }
    }

    Err("Server locale non pronto dopo 60s".to_string())
}

#[tauri::command]
pub async fn check_stt_server(_app: AppHandle) -> String {
    if tokio::net::TcpStream::connect(format!("127.0.0.1:{LOCAL_PORT}"))
        .await
        .is_ok()
    {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

#[tauri::command]
pub async fn start_stt_server(app: AppHandle) -> Result<(), String> {
    start_local_server(app).await
}

// ── Permissions ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn check_screen_recording_permission() -> bool {
    permissions::screen_recording_granted()
}

#[tauri::command]
pub fn request_screen_recording_permission() -> bool {
    permissions::request_screen_recording()
}

#[tauri::command]
pub fn open_permission_settings(pane: String) -> Result<(), String> {
    let pane = match pane.as_str() {
        "microphone" => permissions::PermissionPane::Microphone,
        "screen-recording" => permissions::PermissionPane::ScreenRecording,
        other => return Err(format!("Pannello permessi sconosciuto: {other}")),
    };
    permissions::open_settings_pane(pane)
}

// ── Transcription ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct TranscriptSegment {
    pub id: u32,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub speaker: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TranscriptResult {
    pub task: String,
    pub language: String,
    pub duration: f64,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}

/// Rate unico per registrazione *e* trascrizione: whisper.cpp richiede 16kHz
/// (`read_wav` in `common.cpp`), e registrare direttamente a questo rate
/// (vedi `start_recording`/`stop_recording`) elimina anche il resampling qui
/// sotto per le nuove registrazioni — non è una coincidenza, è la scelta che
/// rende le due esigenze (size su disco, formato richiesto dal server) la
/// stessa cosa.
const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Resample lineare puro-Rust: gestisce rate arbitrari in ingresso, inclusi
/// rapporti non interi (es. 44100 -> 16000). ~15 righe, zero dipendenze esterne
/// (niente `rubato`/`ffmpeg`) — sufficiente per parlato/ASR, non per audio
/// musicale ad alta fedeltà.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// whisper.cpp's `read_wav` accetta solo mono 16kHz 16-bit PCM (`common.cpp`).
/// Le registrazioni sono ora salvate direttamente a `TARGET_SAMPLE_RATE` (vedi
/// `stop_recording`) — per i nuovi file questa conversione è un passthrough;
/// resta necessaria solo per registrazioni legacy a rate nativo del mic.
fn prepare_for_whisper(wav_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = hound::WavReader::new(Cursor::new(wav_bytes)).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let channels = spec.channels as usize;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?
        }
    };

    let mono: Vec<f32> = if channels <= 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    let resampled = resample_linear(&mono, spec.sample_rate, TARGET_SAMPLE_RATE);

    let out_spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut buf, out_spec).map_err(|e| e.to_string())?;
        for s in resampled {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

#[tauri::command]
pub async fn transcribe_recording(
    _app: AppHandle,
    path: String,
) -> Result<TranscriptResult, String> {
    let base_url = format!("http://127.0.0.1:{LOCAL_PORT}");

    let file_bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let converted = tokio::task::spawn_blocking(move || prepare_for_whisper(&file_bytes))
        .await
        .map_err(|e| e.to_string())??;

    let part = reqwest::multipart::Part::bytes(converted)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("language", "it")
        .text("response_format", "verbose_json");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/inference"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let mut result: TranscriptResult = resp.json().await.map_err(|e| e.to_string())?;

    if let Some(timeline) = load_diarization_sidecar(&path).await {
        for segment in result.segments.iter_mut() {
            segment.speaker = dominant_speaker(segment.start, segment.end, &timeline)
                .map(|label| label.as_str().to_string());
        }
    }

    Ok(result)
}

/// Carica il sidecar `<nome registrazione>.diarization.json` scritto da `stop_recording`,
/// se presente — registrazioni più vecchie o solo-microfono non ne hanno uno, e in tal
/// caso `speaker` resta `None` (nessuna regressione rispetto a prima).
async fn load_diarization_sidecar(wav_path: &str) -> Option<Vec<diarization::SpeakerInterval>> {
    let sidecar_path = PathBuf::from(wav_path).with_extension("diarization.json");
    let bytes = tokio::fs::read(&sidecar_path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Etichetta dominante: l'intervallo del timeline con la maggiore sovrapposizione
/// temporale rispetto al segmento trascritto.
fn dominant_speaker(
    start: f64,
    end: f64,
    timeline: &[diarization::SpeakerInterval],
) -> Option<diarization::SpeakerLabel> {
    timeline
        .iter()
        .map(|interval| {
            let overlap = (interval.end.min(end) - interval.start.max(start)).max(0.0);
            (overlap, interval.label)
        })
        .filter(|(overlap, _)| *overlap > 0.0)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, label)| label)
}

// ── Recording ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RecordingStatus {
    pub is_recording: bool,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn start_recording(state: State<'_, RecorderState>) -> Result<(), String> {
    let mut inner = state.0.lock().await;
    if inner.is_recording {
        return Err("Already recording".to_string());
    }

    let mic_buf = inner.mic_samples.clone();
    let sys_buf = inner.sys_samples.clone();

    mic_buf.lock().unwrap().clear();
    sys_buf.lock().unwrap().clear();

    let mic_info = mic::start_mic_capture(mic_buf)?;
    // SCStream (macOS) converte nativamente al rate richiesto — catturare l'audio
    // di sistema già a TARGET_SAMPLE_RATE evita di doverlo ricampionare a valle.
    let sys_capture = match system_audio::start(sys_buf, TARGET_SAMPLE_RATE, mic_info.channels) {
        Ok(capture) => Some(capture),
        Err(e) => {
            eprintln!("Cattura audio di sistema non disponibile: {e}");
            None
        }
    };

    inner.sample_rate = TARGET_SAMPLE_RATE;
    inner.mic_native_rate = mic_info.sample_rate;
    inner.channels = mic_info.channels;
    inner.mic_stream = Some(mic_info.stream);
    inner.sys_capture = sys_capture;
    inner.is_recording = true;
    inner.start_time = Some(std::time::Instant::now());

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, RecorderState>,
    app: AppHandle,
) -> Result<String, String> {
    let (mixed, sample_rate, channels, timeline) = {
        let mut inner = state.0.lock().await;
        if !inner.is_recording {
            return Err("Not recording".to_string());
        }

        inner.mic_stream = None;
        let had_sys_audio = inner.sys_capture.is_some();
        if let Some(mut sys) = inner.sys_capture.take() {
            sys.stop()?;
        }
        inner.is_recording = false;
        inner.start_time = None;

        let mic_raw = inner.mic_samples.lock().unwrap().clone();
        let sys = inner.sys_samples.lock().unwrap().clone();
        let sr = inner.sample_rate;
        let ch = inner.channels;

        // Il mic cattura al rate nativo del device (cpal non lo forza); l'audio
        // di sistema arriva già a `sr` (SCStream lo converte nativamente, vedi
        // start_recording). Riallineiamo il mic a `sr` prima di mixer/diarizzazione
        // — entrambi richiedono buffer allo stesso rate.
        let mic = resample_linear(&mic_raw, inner.mic_native_rate, sr);

        // Solo se è stato catturato anche audio di sistema ha senso stimare un
        // timeline mic-vs-sistema: senza una seconda sorgente sarebbe sempre "mic".
        let timeline = had_sys_audio.then(|| diarization::estimate_timeline(&mic, &sys, sr));

        (mixer::mix(&mic, &sys), sr, ch, timeline)
    };

    let settings = get_stt_settings(app.clone()).await;
    let dir = recordings_dir(&app, &settings);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H.%M.%S");
    let path = dir.join(format!("Registrazione {timestamp}.wav"));

    writer::write_wav(&mixed, &path, sample_rate, channels)?;

    // Sidecar best-effort: la diarizzazione è un arricchimento, non deve mai far
    // fallire la registrazione (il file WAV è già scritto a questo punto).
    if let Some(timeline) = timeline {
        let sidecar_path = path.with_extension("diarization.json");
        match serde_json::to_vec(&timeline) {
            Ok(json) => {
                if let Err(e) = tokio::fs::write(&sidecar_path, json).await {
                    eprintln!("Impossibile scrivere sidecar diarizzazione: {e}");
                }
            }
            Err(e) => eprintln!("Impossibile serializzare timeline diarizzazione: {e}"),
        }
    }

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_recording_status(
    state: State<'_, RecorderState>,
) -> Result<RecordingStatus, String> {
    let inner = state.0.lock().await;
    Ok(RecordingStatus {
        is_recording: inner.is_recording,
        duration_ms: inner
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0),
    })
}

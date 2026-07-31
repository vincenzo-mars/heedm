//! Modello locale, ciclo di vita di whisper-server e trascrizione.

use std::io::Cursor;
use std::path::PathBuf;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use super::{bundled_bin_path, get_stt_settings, local_model_path, save_stt_settings, LOCAL_PORT};
use crate::recorder::audio::{self, TARGET_SAMPLE_RATE};
use crate::recorder::diarization;

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";

// ── Download ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn download_local_model(app: AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    let settings = get_stt_settings(app.clone()).await;

    // whisper-server è bundled come risorsa app — qui si scarica solo il modello
    let model_path = local_model_path(&app, &settings);
    if let Some(parent) = model_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

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

    let mut settings = get_stt_settings(app.clone()).await;
    settings.local_ready = true;
    save_stt_settings(app, settings).await
}

// ── Ciclo di vita del server ──────────────────────────────────────────────────

/// Tiene l'handle del processo whisper-server per poterlo terminare
/// alla chiusura dell'app (altrimenti resta in background dopo il quit).
pub struct WhisperServerState(pub std::sync::Mutex<Option<tokio::process::Child>>);

impl WhisperServerState {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }
}

async fn port_is_open() -> bool {
    tokio::net::TcpStream::connect(format!("127.0.0.1:{LOCAL_PORT}"))
        .await
        .is_ok()
}

async fn start_local_server(app: AppHandle) -> Result<(), String> {
    if port_is_open().await {
        return Ok(());
    }

    let settings = get_stt_settings(app.clone()).await;
    let bin = bundled_bin_path(&app)?;
    let model = local_model_path(&app, &settings);

    if !model.exists() {
        return Err("Modello locale non scaricato".to_string());
    }
    let model = model
        .to_str()
        .ok_or("Percorso del modello non rappresentabile come UTF-8")?;

    let child = tokio::process::Command::new(&bin)
        .args([
            "--model",
            model,
            "--host",
            "127.0.0.1",
            "--port",
            &LOCAL_PORT.to_string(),
        ])
        .spawn()
        .map_err(|e| e.to_string())?;

    match app.state::<WhisperServerState>().0.lock() {
        Ok(mut guard) => *guard = Some(child),
        Err(e) => return Err(format!("Stato del server whisper corrotto: {e}")),
    }

    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if port_is_open().await {
            return Ok(());
        }
    }

    Err("Server locale non pronto dopo 60s".to_string())
}

#[tauri::command]
pub async fn check_stt_server() -> String {
    if port_is_open().await {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

#[tauri::command]
pub async fn start_stt_server(app: AppHandle) -> Result<(), String> {
    start_local_server(app).await
}

// ── Trascrizione ──────────────────────────────────────────────────────────────

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
    // Tempo di elaborazione della trascrizione (non presente nella risposta del
    // server né nei transcript.json vecchi → `default` = None). Popolato da
    // `transcribe_recording` e persistito nel sidecar.
    #[serde(default)]
    pub transcription_ms: Option<u64>,
}

/// whisper.cpp's `read_wav` accetta solo mono 16kHz 16-bit PCM (`common.cpp`).
/// Le registrazioni sono ora salvate direttamente a `TARGET_SAMPLE_RATE` — per i
/// nuovi file questa conversione è un passthrough; resta necessaria solo per
/// registrazioni legacy a rate nativo del mic.
fn prepare_for_whisper(wav_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = hound::WavReader::new(Cursor::new(wav_bytes)).map_err(|e| e.to_string())?;
    let spec = reader.spec();

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

    let mono = audio::to_mono(samples, spec.channels);
    let resampled = audio::resample_linear(&mono, spec.sample_rate, TARGET_SAMPLE_RATE);
    audio::encode_wav(&resampled, TARGET_SAMPLE_RATE, 1)
}

#[tauri::command]
pub async fn transcribe_recording(path: String) -> Result<TranscriptResult, String> {
    let base_url = format!("http://127.0.0.1:{LOCAL_PORT}");
    let started = std::time::Instant::now();

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
    result.transcription_ms = Some(started.elapsed().as_millis() as u64);

    if let Some(timeline) = load_diarization_sidecar(&path).await {
        for segment in result.segments.iter_mut() {
            segment.speaker = dominant_speaker(segment.start, segment.end, &timeline)
                .map(|label| label.as_str().to_string());
        }
    }

    let wav_path_buf = PathBuf::from(&path);
    if let Some(folder) = wav_path_buf.parent() {
        if let Ok(json) = serde_json::to_vec(&result) {
            tokio::fs::write(folder.join("transcript.json"), json)
                .await
                .ok();
        }
    }

    Ok(result)
}

/// Carica il sidecar `<nome registrazione>.diarization.json` scritto da `stop_recording`,
/// se presente — registrazioni più vecchie o solo-microfono non ne hanno uno, e in tal
/// caso `speaker` resta `None`.
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

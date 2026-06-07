use std::path::PathBuf;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::io::AsyncWriteExt;

use crate::recorder::{mic, mixer, system_audio, writer, RecorderState};

// ── Constants ────────────────────────────────────────────────────────────────

const LOCAL_PORT: u16 = 8080;

// Update these URLs to the actual whisper.cpp release assets for your target platform.
#[cfg(target_arch = "aarch64")]
const BIN_URL: &str = "https://github.com/ggerganov/whisper.cpp/releases/download/v1.7.4/whisper-blas-blas-server-osx-arm64.zip";
#[cfg(not(target_arch = "aarch64"))]
const BIN_URL: &str = "https://github.com/ggerganov/whisper.cpp/releases/download/v1.7.4/whisper-blas-blas-server-osx-x64.zip";

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

fn local_bin_path(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    model_dir(app, settings).join("bin").join("whisper-server")
}

fn local_model_path(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    model_dir(app, settings)
        .join("models")
        .join("ggml-large-v3-turbo.bin")
}

fn default_recordings_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .audio_dir()
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
    local_bin_path(&app, &settings).exists() && local_model_path(&app, &settings).exists()
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

    // 1. Download binary zip
    let bin_path = local_bin_path(&app, &settings);
    tokio::fs::create_dir_all(bin_path.parent().unwrap())
        .await
        .map_err(|e| e.to_string())?;
    let tmp_zip = bin_path.parent().unwrap().join("_whisper.zip");

    {
        let resp = client
            .get(BIN_URL)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let total = resp.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(&tmp_zip)
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
                    serde_json::json!({"step":"binary","pct": downloaded * 100 / total}),
                )
                .ok();
            }
        }
    }

    // Extract server binary from zip
    let zip_src = tmp_zip.clone();
    let bin_dest = bin_path.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let f = std::fs::File::open(&zip_src).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            if name == "server" || name.ends_with("/server") {
                let mut out =
                    std::fs::File::create(&bin_dest).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&bin_dest)
                        .map_err(|e| e.to_string())?
                        .permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&bin_dest, perms).map_err(|e| e.to_string())?;
                }
                break;
            }
        }
        std::fs::remove_file(&zip_src).ok();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    // 2. Download model
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

#[tauri::command]
pub async fn start_local_server(app: AppHandle) -> Result<(), String> {
    if tokio::net::TcpStream::connect(format!("127.0.0.1:{LOCAL_PORT}"))
        .await
        .is_ok()
    {
        return Ok(());
    }

    let settings = get_stt_settings(app.clone()).await;
    let bin = local_bin_path(&app, &settings);
    let model = local_model_path(&app, &settings);

    if !bin.exists() || !model.exists() {
        return Err("Modello locale non scaricato".to_string());
    }

    tokio::process::Command::new(&bin)
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

#[tauri::command]
pub async fn transcribe_recording(_app: AppHandle, path: String) -> Result<TranscriptResult, String> {
    let base_url = format!("http://127.0.0.1:{LOCAL_PORT}");

    let file_bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1")
        .text("language", "it")
        .text("response_format", "verbose_json");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/v1/audio/transcriptions"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    resp.json::<TranscriptResult>().await.map_err(|e| e.to_string())
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
    let sys_capture = system_audio::start(sys_buf, mic_info.sample_rate, mic_info.channels).ok();

    inner.sample_rate = mic_info.sample_rate;
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
    let (mixed, sample_rate, channels) = {
        let mut inner = state.0.lock().await;
        if !inner.is_recording {
            return Err("Not recording".to_string());
        }

        inner.mic_stream = None;
        if let Some(mut sys) = inner.sys_capture.take() {
            sys.stop()?;
        }
        inner.is_recording = false;
        inner.start_time = None;

        let mic = inner.mic_samples.lock().unwrap().clone();
        let sys = inner.sys_samples.lock().unwrap().clone();
        let sr = inner.sample_rate;
        let ch = inner.channels;
        (mixer::mix(&mic, &sys), sr, ch)
    };

    let settings = get_stt_settings(app.clone()).await;
    let dir = recordings_dir(&app, &settings);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H.%M.%S");
    let path = dir.join(format!("Registrazione {timestamp}.wav"));

    writer::write_wav(&mixed, &path, sample_rate, channels)?;
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

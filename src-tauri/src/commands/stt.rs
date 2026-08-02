//! Modello locale, ciclo di vita di whisper-server e trascrizione.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use super::{bundled_bin_path, get_stt_settings, local_model_path, save_stt_settings, LOCAL_PORT};

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";

// ── Download ──────────────────────────────────────────────────────────────────

/// Scarica il modello in `tmp_path`, emettendo `download-progress`. Isolata da
/// `download_local_model` per poter ripulire il file temporaneo su qualunque
/// errore a metà streaming, senza duplicare la logica di cleanup.
async fn stream_model_to(
    client: &reqwest::Client,
    app: &AppHandle,
    tmp_path: &PathBuf,
) -> Result<(), String> {
    let resp = client
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(tmp_path)
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

    Ok(())
}

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

    // Si scrive su un file `.part` e si fa rename atomico solo a download
    // completo: se l'app viene chiusa o crasha a metà, resta un .part
    // troncato invece di un .bin che `model.exists()` scambierebbe per valido.
    let tmp_path = PathBuf::from(format!("{}.part", model_path.display()));

    if let Err(e) = stream_model_to(&client, &app, &tmp_path).await {
        tokio::fs::remove_file(&tmp_path).await.ok();
        return Err(e);
    }

    tokio::fs::rename(&tmp_path, &model_path)
        .await
        .map_err(|e| e.to_string())?;

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

    // Flash attention e numero di thread non hanno UI: si deducono dalla
    // macchina. `-fa` esiste solo dalle build recenti di whisper.cpp (vedi
    // scripts/build-whisper-server.sh, tag pinnato).
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(2).max(1))
        .unwrap_or(4);

    let child = tokio::process::Command::new(&bin)
        .args([
            "--model",
            model,
            "--host",
            "127.0.0.1",
            "--port",
            &LOCAL_PORT.to_string(),
            "--threads",
            &threads.to_string(),
            "--flash-attn",
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

/// Secondi di attesa dopo il kill prima di arrendersi: il processo può tenere
/// il socket ancora un istante dopo che `wait()` è tornato.
const PORT_RELEASE_TIMEOUT_SECS: u64 = 5;

async fn stop_local_server(app: &AppHandle) -> Result<(), String> {
    // Il MutexGuard vive solo dentro il match: va tolto prima di qualsiasi
    // `.await` sul child (kill/wait), altrimenti il lock resterebbe tenuto
    // attraverso un punto di sospensione.
    let child = match app.state::<WhisperServerState>().0.lock() {
        Ok(mut guard) => guard.take(),
        Err(e) => return Err(format!("Stato del server whisper corrotto: {e}")),
    };

    match child {
        Some(mut child) => {
            child.start_kill().map_err(|e| e.to_string())?;
            child.wait().await.map_err(|e| e.to_string())?;
        }
        None => {
            if port_is_open().await {
                return Err(
                    "La porta 8080 è occupata da un processo che Heedm non ha avviato: chiudilo manualmente prima di riprovare".to_string(),
                );
            }
            return Ok(());
        }
    }

    if !port_is_open().await {
        return Ok(());
    }
    for _ in 0..PORT_RELEASE_TIMEOUT_SECS {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if !port_is_open().await {
            return Ok(());
        }
    }

    Err(format!(
        "La porta 8080 resta occupata {PORT_RELEASE_TIMEOUT_SECS}s dopo lo stop del server"
    ))
}

#[tauri::command]
pub async fn stop_stt_server(app: AppHandle) -> Result<(), String> {
    stop_local_server(&app).await
}

#[tauri::command]
pub async fn restart_stt_server(app: AppHandle) -> Result<(), String> {
    stop_local_server(&app).await?;
    start_local_server(app).await
}

#[tauri::command]
pub async fn delete_local_model(app: AppHandle) -> Result<(), String> {
    // Il processo tiene il .bin aperto: va fermato prima di cancellare,
    // altrimenti su alcuni filesystem la delete fallisce o lascia un file
    // fantasma finché il processo non lo rilascia da solo.
    stop_local_server(&app).await?;

    let settings = get_stt_settings(app.clone()).await;
    let model_path = local_model_path(&app, &settings);
    match tokio::fs::remove_file(&model_path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
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

/// La diarizzazione la fa whisper: su un file **stereo** con `diarize=true`
/// confronta l'energia dei due canali segmento per segmento e ritorna `"0"` per
/// il sinistro (microfono) e `"1"` per il destro (audio di sistema), oppure
/// `"?"` quando le due energie sono a meno del 10% l'una dall'altra.
/// L'ambiguità non è una terza etichetta: viene normalizzata a `None`, lo stesso
/// stato dei file mono (import, registrazioni senza audio di sistema).
fn normalize_speaker(speaker: Option<String>) -> Option<String> {
    match speaker.as_deref() {
        Some("0") | Some("1") => speaker,
        _ => None,
    }
}

/// Traccia persistente di un fallimento, scritta accanto al WAV così
/// `list_recordings` può esporre lo stato "fallito" anche dopo un riavvio
/// dell'app (un errore di rete/parsing altrimenti non lascerebbe traccia sul
/// disco). Rimossa da `transcribe_recording` al primo rilancio riuscito.
const TRANSCRIPT_ERROR_FILE: &str = "transcript_error.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct TranscriptError {
    pub message: String,
}

async fn run_transcription(path: &str) -> Result<TranscriptResult, String> {
    let base_url = format!("http://127.0.0.1:{LOCAL_PORT}");
    let started = std::time::Instant::now();

    // I file prodotti da Heedm sono già nel formato che whisper.cpp richiede
    // (16kHz 16-bit PCM), quindi vanno inviati così come sono: nessuna
    // conversione intermedia, nessuna seconda copia in memoria.
    let file_bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("language", "it")
        .text("response_format", "verbose_json")
        // Ignorato dal server sui file mono, dove `pcmf32s` ha un solo canale.
        .text("diarize", "true");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/inference"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let mut result: TranscriptResult = resp.json().await.map_err(|e| e.to_string())?;
    result.transcription_ms = Some(started.elapsed().as_millis() as u64);
    for segment in result.segments.iter_mut() {
        segment.speaker = normalize_speaker(segment.speaker.take());
    }

    let wav_path_buf = PathBuf::from(path);
    if let Some(folder) = wav_path_buf.parent() {
        if let Ok(json) = serde_json::to_vec(&result) {
            tokio::fs::write(folder.join("transcript.json"), json)
                .await
                .ok();
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn transcribe_recording(path: String) -> Result<TranscriptResult, String> {
    let result = run_transcription(&path).await;

    if let Some(folder) = Path::new(&path).parent() {
        let error_sidecar = folder.join(TRANSCRIPT_ERROR_FILE);
        match &result {
            // Un rilancio riuscito deve cancellare l'errore di un tentativo
            // precedente: altrimenti `list_recordings` continuerebbe a
            // mostrare "fallito" su un record che ora ha un transcript valido.
            Ok(_) => {
                tokio::fs::remove_file(&error_sidecar).await.ok();
            }
            Err(message) => {
                let error = TranscriptError {
                    message: message.clone(),
                };
                if let Ok(json) = serde_json::to_vec(&error) {
                    tokio::fs::write(&error_sidecar, json).await.ok();
                }
            }
        }
    }

    result
}

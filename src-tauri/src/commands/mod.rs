//! Tauri commands esposti al frontend, divisi per dominio:
//! qui impostazioni, percorsi e permessi OS; `stt` per modello, server e
//! trascrizione; `recording` per cattura, import e listing.

pub mod recording;
pub mod stt;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::permissions;

pub(crate) const LOCAL_PORT: u16 = 8080;

// ── Settings ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SttSettings {
    pub local_ready: bool,
    pub configured: bool,
    pub model_dir: Option<String>,
    pub recordings_dir: Option<String>,
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

pub(crate) fn bundled_bin_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve("whisper-server", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())
}

pub(crate) fn local_model_path(app: &AppHandle, settings: &SttSettings) -> PathBuf {
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

pub(crate) fn recordings_dir(app: &AppHandle, settings: &SttSettings) -> PathBuf {
    match &settings.recordings_dir {
        Some(dir) => PathBuf::from(dir),
        None => default_recordings_dir(app),
    }
}

#[tauri::command]
pub async fn get_stt_settings(app: AppHandle) -> SttSettings {
    let path = settings_path(&app);
    let mut settings: SttSettings = tokio::fs::read_to_string(&path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // local_ready persistito è solo l'ultimo valore noto: la verità è il file
    // sul disco (l'utente può cancellarlo a mano, un download può essere stato
    // interrotto). Va sempre riverificato, mai fidarsi del flag salvato.
    settings.local_ready = local_model_path(&app, &settings).exists();
    settings
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

// ── Permessi OS ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn check_screen_recording_permission() -> bool {
    permissions::screen_recording_granted()
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

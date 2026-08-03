//! Streaming HTTP condiviso fra il download del modello whisper e quello del
//! modello LLM. Estratto da `stt.rs` quando è arrivato un secondo download con
//! lo stesso invariante: scrittura su `.part` + rename atomico solo a
//! download completo, e cleanup del temporaneo su qualunque errore a metà
//! streaming, altrimenti un crash a metà lascerebbe un file troncato che
//! `.exists()` scambierebbe per un modello valido.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// Scarica `url` in `tmp_path`, emettendo `event` con `{"step": step, "pct": ..}`
/// a ogni chunk ricevuto.
async fn stream_to(
    client: &reqwest::Client,
    app: &AppHandle,
    url: &str,
    tmp_path: &Path,
    event: &str,
    step: &str,
) -> Result<(), String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let resp = resp.error_for_status().map_err(|e| e.to_string())?;
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
                event,
                serde_json::json!({"step": step, "pct": downloaded * 100 / total}),
            )
            .ok();
        }
    }

    Ok(())
}

/// Scarica `url` in `final_path`, passando per un file `.part` accanto ad
/// esso. Su errore, il `.part` viene ripulito e l'errore propagato.
pub(crate) async fn download_to_file(
    app: &AppHandle,
    url: &str,
    final_path: &PathBuf,
    event: &str,
    step: &str,
) -> Result<(), String> {
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    let client = reqwest::Client::new();
    let tmp_path = PathBuf::from(format!("{}.part", final_path.display()));

    if let Err(e) = stream_to(&client, app, url, &tmp_path, event, step).await {
        tokio::fs::remove_file(&tmp_path).await.ok();
        return Err(e);
    }

    tokio::fs::rename(&tmp_path, final_path)
        .await
        .map_err(|e| e.to_string())?;

    app.emit(event, serde_json::json!({"step": "done", "pct": 100}))
        .ok();

    Ok(())
}

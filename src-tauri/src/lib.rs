mod commands;
mod permissions;
mod recorder;

use commands::stt::WhisperServerState;
use recorder::RecorderState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(RecorderState::new())
        .manage(WhisperServerState::new())
        .invoke_handler(tauri::generate_handler![
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::get_recording_status,
            commands::recording::import_audio_file,
            commands::recording::list_recordings,
            commands::stt::transcribe_recording,
            commands::stt::check_stt_server,
            commands::stt::start_stt_server,
            commands::stt::stop_stt_server,
            commands::stt::restart_stt_server,
            commands::stt::download_local_model,
            commands::stt::delete_local_model,
            commands::get_stt_settings,
            commands::save_stt_settings,
            commands::get_local_model_path,
            commands::get_recordings_dir,
            commands::check_screen_recording_permission,
            commands::open_permission_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            let child = app_handle
                .state::<WhisperServerState>()
                .0
                .lock()
                .ok()
                .and_then(|mut guard| guard.take());
            if let Some(mut child) = child {
                let _ = child.start_kill();
            }
        }
    });
}

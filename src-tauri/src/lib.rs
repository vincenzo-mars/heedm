mod commands;
mod permissions;
mod recorder;

use recorder::RecorderState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(RecorderState::new())
        .manage(commands::WhisperServerState::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::get_recording_status,
            commands::transcribe_recording,
            commands::check_stt_server,
            commands::start_stt_server,
            commands::get_stt_settings,
            commands::save_stt_settings,
            commands::get_local_model_status,
            commands::get_local_model_path,
            commands::get_recordings_dir,
            commands::pick_directory,
            commands::download_local_model,
            commands::start_local_server,
            commands::check_screen_recording_permission,
            commands::request_screen_recording_permission,
            commands::open_permission_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            if let Some(mut child) = app_handle
                .state::<commands::WhisperServerState>()
                .0
                .lock()
                .unwrap()
                .take()
            {
                let _ = child.start_kill();
            }
        }
    });
}

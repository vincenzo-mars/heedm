mod commands;
mod recorder;

use recorder::RecorderState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(RecorderState::new())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

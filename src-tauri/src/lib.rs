mod commands;
mod permissions;
mod recorder;

use commands::llm::LlamaServerState;
use commands::stt::WhisperServerState;
use recorder::RecorderState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .manage(RecorderState::new())
        .manage(WhisperServerState::new())
        .manage(LlamaServerState::new())
        .invoke_handler(tauri::generate_handler![
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::import_audio_file,
            commands::recording::list_recordings,
            commands::stt::transcribe_recording,
            commands::stt::check_stt_server,
            commands::stt::start_stt_server,
            commands::stt::stop_stt_server,
            commands::stt::restart_stt_server,
            commands::stt::download_local_model,
            commands::stt::delete_local_model,
            commands::llm::get_system_memory_gb,
            commands::llm::search_hf_models,
            commands::llm::get_hf_model_files,
            commands::llm::set_llm_model,
            commands::llm::download_llm_model,
            commands::llm::check_llm_server,
            commands::llm::start_llm_server,
            commands::llm::stop_llm_server,
            commands::llm::restart_llm_server,
            commands::llm::clear_llm_cache,
            commands::llm::read_recording_notes,
            commands::llm::write_recording_notes,
            commands::llm::delete_recording_notes,
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
            commands::server::kill_tracked(&app_handle.state::<WhisperServerState>().0);
            commands::server::kill_tracked(&app_handle.state::<LlamaServerState>().0);
        }
    });
}

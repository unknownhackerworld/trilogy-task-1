mod audio;
mod asr;
mod commands;
mod pipeline;
mod state;
mod translation;

use state::AppState;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::start_translation,
            commands::stop_translation,
            commands::set_languages,
            commands::get_pipeline_status,
            commands::get_settings,
            commands::save_settings,
            commands::open_overlay,
            commands::close_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

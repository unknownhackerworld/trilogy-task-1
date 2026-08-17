use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tracing::info;

use crate::audio::{AudioCapture, AudioDevice};
use crate::pipeline::{Pipeline, PipelineStatus};
use crate::state::{AppSettings, AppState};

#[derive(Debug, Serialize)]
pub struct CommandError {
    message: String,
}

impl From<anyhow::Error> for CommandError {
    fn from(e: anyhow::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CommandError>;

/// List all audio devices available for capture.
#[tauri::command]
pub fn list_audio_devices() -> CmdResult<Vec<AudioDevice>> {
    let devices = AudioCapture::list_devices().map_err(CommandError::from)?;
    Ok(devices)
}

/// Start the translation pipeline.
#[tauri::command]
pub fn start_translation(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    device_name: String,
    sample_rate: u32,
    channels: u16,
) -> CmdResult<()> {
    let mut pipeline_guard = state.pipeline.lock();

    // Stop existing pipeline if running
    if let Some(ref mut existing) = *pipeline_guard {
        existing.stop();
    }

    let settings = state.settings.lock().clone();

    let device = AudioDevice {
        id: device_id,
        name: device_name.clone(),
        process_name: device_name,
        sample_rate,
        channels,
        is_active: true,
    };

    let pipeline = Pipeline::start(app_handle, device, &settings).map_err(CommandError::from)?;
    *pipeline_guard = Some(pipeline);

    Ok(())
}

/// Stop the translation pipeline.
#[tauri::command]
pub fn stop_translation(state: State<'_, AppState>) -> CmdResult<()> {
    let mut pipeline_guard = state.pipeline.lock();
    if let Some(ref mut pipeline) = *pipeline_guard {
        pipeline.stop();
    }
    *pipeline_guard = None;
    Ok(())
}

/// Change language pair while running.
#[tauri::command]
pub fn set_languages(
    state: State<'_, AppState>,
    source_lang: String,
    target_lang: String,
) -> CmdResult<()> {
    // Validate language codes
    let valid_codes = [
        "en", "ta", "hi", "es", "fr", "de", "ja", "ko", "zh-CN", "ar", "pt", "ru", "it", "te",
        "bn", "vi", "th", "id", "nl", "tr",
    ];

    if !valid_codes.contains(&source_lang.as_str()) || !valid_codes.contains(&target_lang.as_str())
    {
        return Err(CommandError {
            message: "Invalid language code".to_string(),
        });
    }

    let mut settings = state.settings.lock();
    settings.source_lang = source_lang;
    settings.target_lang = target_lang;

    Ok(())
}

/// Get current pipeline status.
#[tauri::command]
pub fn get_pipeline_status(state: State<'_, AppState>) -> CmdResult<PipelineStatus> {
    let pipeline_guard = state.pipeline.lock();
    match &*pipeline_guard {
        Some(pipeline) => Ok(pipeline.status()),
        None => Ok(PipelineStatus {
            state: "idle".to_string(),
            duration_secs: 0,
            sentences_transcribed: 0,
            sentences_translated: 0,
            current_level: 0.0,
        }),
    }
}

/// Get current settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CmdResult<AppSettings> {
    let settings = state.settings.lock().clone();
    Ok(settings)
}

/// Save settings.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> CmdResult<()> {
    AppState::save_settings(&settings).map_err(CommandError::from)?;
    *state.settings.lock() = settings;
    Ok(())
}

/// Open overlay window.
#[tauri::command]
pub async fn open_overlay(app_handle: AppHandle) -> CmdResult<()> {
    // Hide main window
    if let Some(main_window) = app_handle.get_webview_window("main") {
        let _ = main_window.hide();
    }

    // Create overlay window
    let _overlay = WebviewWindowBuilder::new(
        &app_handle,
        "overlay",
        WebviewUrl::App("index.html#/overlay".into()),
    )
    .title("")
    .inner_size(560.0, 140.0)
    .position(
        (app_handle.primary_monitor().unwrap().unwrap().size().width as f64 - 560.0) / 2.0,
        app_handle.primary_monitor().unwrap().unwrap().size().height as f64 - 200.0,
    )
    .decorations(false)
    .always_on_top(true)
    .transparent(true)
    .resizable(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| CommandError {
        message: format!("Failed to create overlay window: {}", e),
    })?;

    info!("Overlay window opened");
    Ok(())
}

/// Close overlay and return to control panel.
#[tauri::command]
pub async fn close_overlay(app_handle: AppHandle) -> CmdResult<()> {
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        let _ = overlay.close();
    }
    if let Some(main_window) = app_handle.get_webview_window("main") {
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }
    Ok(())
}

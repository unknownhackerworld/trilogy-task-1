use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::pipeline::Pipeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub asr_engine: String,
    pub translation_engine: String,
    pub source_lang: String,
    pub target_lang: String,
    pub whisper_model: String,
    pub deepgram_api_key: String,
    pub overlay_show_source: bool,
    pub overlay_opacity: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            asr_engine: "deepgram".to_string(),
            translation_engine: "libre".to_string(),
            source_lang: "en".to_string(),
            target_lang: "ta".to_string(),
            whisper_model: "large-v3-turbo".to_string(),
            deepgram_api_key: String::new(),
            overlay_show_source: true,
            overlay_opacity: 0.92,
        }
    }
}

pub struct AppState {
    pub pipeline: Arc<Mutex<Option<Pipeline>>>,
    pub settings: Arc<Mutex<AppSettings>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut settings = Self::load_settings().unwrap_or_default();

        // Allow overriding API keys via environment variables at runtime.
        // This means setting DEEPGRAM_API_KEY in the shell before running
        // the app will always take precedence over the saved settings file.
        if let Ok(key) = std::env::var("DEEPGRAM_API_KEY") {
            if !key.is_empty() {
                settings.deepgram_api_key = key;
            }
        }

        Self {
            pipeline: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(settings)),
        }
    }

    fn load_settings() -> Option<AppSettings> {
        let config_dir = dirs_config_path()?;
        let settings_path = config_dir.join("settings.json");
        let content = std::fs::read_to_string(settings_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save_settings(settings: &AppSettings) -> anyhow::Result<()> {
        let config_dir = dirs_config_path().ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        std::fs::create_dir_all(&config_dir)?;
        let settings_path = config_dir.join("settings.json");
        let content = serde_json::to_string_pretty(settings)?;
        std::fs::write(settings_path, content)?;
        Ok(())
    }
}

fn dirs_config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("speech-translator"))
}

/// Helper to get config directory (used by Whisper model downloads, etc.)
pub fn app_data_dir() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|p| p.join("speech-translator"))
}

// dirs crate dependency — add to Cargo.toml

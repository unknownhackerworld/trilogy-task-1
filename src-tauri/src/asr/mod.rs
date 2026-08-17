pub mod deepgram;
pub mod whisper;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Events emitted by ASR engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AsrEvent {
    /// Partial transcription (still speaking)
    Interim { text: String },
    /// Final transcription (sentence boundary detected)
    Final { text: String },
    /// Error from ASR engine
    Error { message: String },
}

/// Trait for all ASR backends.
pub trait AsrEngine: Send {
    /// Feed raw 16kHz mono i16 PCM audio.
    fn feed_audio(&mut self, samples: &[i16]) -> Result<Vec<AsrEvent>>;

    /// Flush remaining audio buffer (call on stop).
    fn flush(&mut self) -> Result<Vec<AsrEvent>>;

    /// Change recognition language.
    fn set_language(&mut self, language: &str);
}

/// Create ASR engine based on configuration.
/// `api_key` is required for "deepgram", ignored for "whisper".
pub fn create_asr_engine(
    engine_type: &str,
    language: &str,
    model_size: &str,
    api_key: Option<&str>,
) -> Result<Box<dyn AsrEngine>> {
    match engine_type {
        "deepgram" => {
            let key = api_key.ok_or_else(|| {
                anyhow::anyhow!("Deepgram API key is required. Set DEEPGRAM_API_KEY in your .env file.")
            })?;
            let engine = deepgram::DeepgramAsr::new(key, language)?;
            Ok(Box::new(engine))
        }
        "whisper" | _ => {
            let engine = whisper::WhisperAsr::new(model_size, language)?;
            Ok(Box::new(engine))
        }
    }
}

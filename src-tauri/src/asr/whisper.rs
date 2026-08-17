use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tracing::{info, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{AsrEngine, AsrEvent};
use crate::state::app_data_dir;

// Tuned for large-v3-turbo: it's more sensitive than smaller models
const SILENCE_THRESHOLD_RMS: f32 = 400.0;
const SILENCE_DURATION_SAMPLES: usize = 12800; // 800ms at 16kHz
const MIN_AUDIO_SAMPLES: usize = 8000;         // 500ms minimum
const MAX_AUDIO_SAMPLES: usize = 240000;       // 15s maximum

pub struct WhisperAsr {
    ctx: WhisperContext,
    language: String,
    audio_buffer: Vec<f32>,
    silence_counter: usize,
    has_speech: bool,
    use_gpu: bool,
}

impl WhisperAsr {
    pub fn new(model_size: &str, language: &str) -> Result<Self> {
        let model_path = get_model_path(model_size)?;

        info!("Loading Whisper model: {} from {:?}", model_size, model_path);

        // Enable GPU acceleration — whisper.cpp will use CUDA if available,
        // fall back to CPU automatically if CUDA is not found at runtime
        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(true);
        ctx_params.gpu_device(0); // First GPU (NVIDIA GPU index 0)

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow!("Invalid model path"))?,
            ctx_params,
        )
        .map_err(|e| anyhow!("Failed to load Whisper model '{}': {}", model_size, e))?;

        info!("Whisper model loaded (GPU: enabled, device: 0)");

        Ok(Self {
            ctx,
            language: language.to_string(),
            audio_buffer: Vec::with_capacity(MAX_AUDIO_SAMPLES),
            silence_counter: 0,
            has_speech: false,
            use_gpu: true,
        })
    }

    fn transcribe(&self, audio: &[f32]) -> Result<String> {
        // BeamSearch gives better accuracy than Greedy — worth the small latency
        // cost for large-v3-turbo since the model itself is the bottleneck
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: 1.0,
        });

        // Language: "auto" lets Whisper detect the language itself
        let lang = if self.language == "auto" {
            None
        } else {
            Some(self.language.as_str())
        };
        params.set_language(lang);

        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        // Don't force single segment — large-v3-turbo handles long audio well
        params.set_single_segment(false);
        params.set_no_context(false);
        // Temperature fallback for robustness
        params.set_temperature(0.0);
        params.set_temperature_inc(0.2);

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| anyhow!("Whisper state error: {}", e))?;

        state
            .full(params, audio)
            .map_err(|e| anyhow!("Whisper inference failed: {}", e))?;

        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(segment_text) = segment.to_str() {
                    let trimmed = segment_text.trim();
                    // Skip hallucination artifacts common in silence
                    if !trimmed.is_empty()
                        && trimmed != "[BLANK_AUDIO]"
                        && trimmed != "(Music)"
                        && !trimmed.starts_with("[Music")
                    {
                        text.push_str(trimmed);
                        text.push(' ');
                    }
                }
            }
        }

        Ok(text.trim().to_string())
    }

    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }
}

impl AsrEngine for WhisperAsr {
    fn feed_audio(&mut self, samples: &[i16]) -> Result<Vec<AsrEvent>> {
        let mut events = Vec::new();

        // Convert i16 → normalized f32 (whisper.cpp expects -1.0 to 1.0)
        let float_samples: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();

        let rms = Self::calculate_rms(&float_samples) * 32768.0;

        if rms > SILENCE_THRESHOLD_RMS {
            self.has_speech = true;
            self.silence_counter = 0;
        } else {
            self.silence_counter += samples.len();
        }

        self.audio_buffer.extend_from_slice(&float_samples);

        let should_transcribe = if self.has_speech
            && self.silence_counter >= SILENCE_DURATION_SAMPLES
        {
            self.audio_buffer.len() >= MIN_AUDIO_SAMPLES
        } else {
            self.audio_buffer.len() >= MAX_AUDIO_SAMPLES
        };

        if should_transcribe && self.has_speech {
            // Trim trailing silence so Whisper doesn't hallucinate on it
            let speech_end = self.audio_buffer.len().saturating_sub(self.silence_counter);
            let end = speech_end.max(MIN_AUDIO_SAMPLES.min(self.audio_buffer.len()));
            let audio_slice = self.audio_buffer[..end].to_vec();

            match self.transcribe(&audio_slice) {
                Ok(text) if !text.is_empty() => {
                    events.push(AsrEvent::Final { text });
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Transcription failed: {}", e);
                    events.push(AsrEvent::Error {
                        message: format!("Transcription error: {}", e),
                    });
                }
            }

            self.audio_buffer.clear();
            self.has_speech = false;
            self.silence_counter = 0;
        }

        Ok(events)
    }

    fn flush(&mut self) -> Result<Vec<AsrEvent>> {
        let mut events = Vec::new();

        if self.has_speech && self.audio_buffer.len() >= MIN_AUDIO_SAMPLES {
            let audio = self.audio_buffer.clone();
            if let Ok(text) = self.transcribe(&audio) {
                if !text.is_empty() {
                    events.push(AsrEvent::Final { text });
                }
            }
        }

        self.audio_buffer.clear();
        self.has_speech = false;
        self.silence_counter = 0;

        Ok(events)
    }

    fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
        self.audio_buffer.clear();
        self.has_speech = false;
        self.silence_counter = 0;
    }
}

/// Resolve model filename and verify it exists.
/// Expected path: %LOCALAPPDATA%\speech-translator\models\ggml-<name>.bin
fn get_model_path(model_size: &str) -> Result<PathBuf> {
    let data_dir =
        app_data_dir().ok_or_else(|| anyhow!("Cannot determine app data directory"))?;
    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir)?;

    let filename = format!("ggml-{}.bin", model_size);
    let model_path = models_dir.join(&filename);

    if model_path.exists() {
        return Ok(model_path);
    }

    let download_url = model_download_url(model_size);

    Err(anyhow!(
        "Whisper model file not found: {:?}\n\
         Download it with:\n\
         curl -L -o \"{path}\" \"{url}\"",
        model_path,
        path = model_path.display(),
        url = download_url,
    ))
}

/// Returns the Hugging Face download URL for a given model size.
fn model_download_url(model_size: &str) -> &'static str {
    match model_size {
        "large-v3-turbo" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"
        }
        "large-v3-turbo-q5_0" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin"
        }
        "large-v3" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"
        }
        "medium" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
        }
        "small" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        }
        "base" => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
        }
        _ => "https://huggingface.co/ggerganov/whisper.cpp/tree/main",
    }
}

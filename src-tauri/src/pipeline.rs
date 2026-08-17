use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tracing::{error, info};

use crate::asr::{create_asr_engine, AsrEvent};
use crate::audio::{AudioCapture, AudioDevice};
use crate::state::AppSettings;
use crate::translation::create_translation_engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub state: String,
    pub duration_secs: u64,
    pub sentences_transcribed: u32,
    pub sentences_translated: u32,
    pub current_level: f32,
}

/// Events emitted to the frontend via Tauri event system.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum PipelineEvent {
    /// Partial transcription (still speaking)
    Interim { text: String },
    /// ASR finished — fires immediately, before translation
    Transcribed { id: u32, text: String },
    /// Translation arrived for a previously transcribed sentence
    Translated { id: u32, source_text: String, translated_text: String },
    /// Audio level update (for UI meter)
    AudioLevel { level: f32 },
    /// Error from any pipeline component
    Error { message: String },
    /// Pipeline state changed
    StateChange { state: String },
}

pub struct Pipeline {
    running: Arc<AtomicBool>,
    capture: AudioCapture,
    worker_handle: Option<thread::JoinHandle<()>>,
    start_time: Instant,
    pub sentences_transcribed: Arc<std::sync::atomic::AtomicU32>,
    pub sentences_translated: Arc<std::sync::atomic::AtomicU32>,
}

impl Pipeline {
    /// Start the full pipeline: audio capture → ASR → translation → UI events.
    pub fn start(
        app_handle: AppHandle,
        device: AudioDevice,
        settings: &AppSettings,
    ) -> Result<Self> {
        let mut capture = AudioCapture::new();
        capture.start(&device.id)?;

        let running = Arc::new(AtomicBool::new(true));
        let sentences_transcribed = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let sentences_translated = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let running_clone = running.clone();
        let receiver = capture.receiver();
        let source_lang = settings.source_lang.clone();
        let target_lang = settings.target_lang.clone();
        let asr_engine_type = settings.asr_engine.clone();
        let whisper_model = settings.whisper_model.clone();
        let deepgram_api_key = settings.deepgram_api_key.clone();
        let translation_engine_type = settings.translation_engine.clone();
        let transcribed_counter = sentences_transcribed.clone();
        let translated_counter = sentences_translated.clone();

        let worker_handle = thread::spawn(move || {
            if let Err(e) = pipeline_worker(
                app_handle,
                receiver,
                running_clone,
                &asr_engine_type,
                &source_lang,
                &target_lang,
                &whisper_model,
                &deepgram_api_key,
                &translation_engine_type,
                transcribed_counter,
                translated_counter,
            ) {
                error!("Pipeline worker error: {}", e);
            }
        });

        info!("Pipeline started: {} → {} (ASR: {}, Translation: {})",
            settings.source_lang, settings.target_lang,
            settings.asr_engine, settings.translation_engine);

        Ok(Self {
            running,
            capture,
            worker_handle: Some(worker_handle),
            start_time: Instant::now(),
            sentences_transcribed,
            sentences_translated,
        })
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.capture.stop();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        info!("Pipeline stopped");
    }

    pub fn status(&self) -> PipelineStatus {
        PipelineStatus {
            state: if self.running.load(Ordering::SeqCst) {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            duration_secs: self.start_time.elapsed().as_secs(),
            sentences_transcribed: self.sentences_transcribed.load(Ordering::Relaxed),
            sentences_translated: self.sentences_translated.load(Ordering::Relaxed),
            current_level: 0.0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

#[allow(clippy::too_many_arguments)]
fn pipeline_worker(
    app_handle: AppHandle,
    receiver: async_channel::Receiver<crate::audio::AudioChunk>,
    running: Arc<AtomicBool>,
    asr_engine_type: &str,
    source_lang: &str,
    target_lang: &str,
    whisper_model: &str,
    deepgram_api_key: &str,
    translation_engine_type: &str,
    transcribed_counter: Arc<std::sync::atomic::AtomicU32>,
    translated_counter: Arc<std::sync::atomic::AtomicU32>,
) -> Result<()> {
    // Initialize ASR engine — pass API key for cloud engines
    let api_key = if deepgram_api_key.is_empty() { None } else { Some(deepgram_api_key) };
    let mut asr = create_asr_engine(asr_engine_type, source_lang, whisper_model, api_key)?;

    // Initialize translation engine
    let translator = create_translation_engine(translation_engine_type, None)?;

    let mut level_throttle = 0u32;
    let mut sentence_id: u32 = 0;

    while running.load(Ordering::SeqCst) {
        match receiver.recv_blocking() {
            Ok(chunk) => {
                // Emit audio level (throttled to ~10Hz)
                level_throttle += 1;
                if level_throttle % 3 == 0 {
                    let _ = app_handle.emit("pipeline-event", PipelineEvent::AudioLevel {
                        level: chunk.level,
                    });
                }

                // Feed to ASR
                match asr.feed_audio(&chunk.data) {
                    Ok(events) => {
                        for event in events {
                            match event {
                                AsrEvent::Interim { text } => {
                                    let _ = app_handle.emit("pipeline-event", PipelineEvent::Interim { text });
                                }
                                AsrEvent::Final { ref text } => {
                                    sentence_id += 1;
                                    let sid = sentence_id;
                                    transcribed_counter.fetch_add(1, Ordering::Relaxed);

                                    // 1. Emit transcription immediately — UI shows it right away
                                    let _ = app_handle.emit("pipeline-event", PipelineEvent::Transcribed {
                                        id: sid,
                                        text: text.clone(),
                                    });

                                    // 2. Translate and emit when done
                                    match translator.translate(text, source_lang, target_lang) {
                                        Ok(result) => {
                                            translated_counter.fetch_add(1, Ordering::Relaxed);
                                            let _ = app_handle.emit("pipeline-event", PipelineEvent::Translated {
                                                id: sid,
                                                source_text: result.source_text,
                                                translated_text: result.translated_text,
                                            });
                                        }
                                        Err(e) => {
                                            let _ = app_handle.emit("pipeline-event", PipelineEvent::Error {
                                                message: format!("Translation failed: {}", e),
                                            });
                                        }
                                    }
                                }
                                AsrEvent::Error { message } => {
                                    let _ = app_handle.emit("pipeline-event", PipelineEvent::Error { message });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = app_handle.emit("pipeline-event", PipelineEvent::Error {
                            message: format!("ASR error: {}", e),
                        });
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Flush remaining audio
    if let Ok(events) = asr.flush() {
        for event in events {
            if let AsrEvent::Final { ref text } = event {
                sentence_id += 1;
                let sid = sentence_id;
                let _ = app_handle.emit("pipeline-event", PipelineEvent::Transcribed {
                    id: sid,
                    text: text.clone(),
                });
                if let Ok(result) = translator.translate(text, source_lang, target_lang) {
                    let _ = app_handle.emit("pipeline-event", PipelineEvent::Translated {
                        id: sid,
                        source_text: result.source_text,
                        translated_text: result.translated_text,
                    });
                }
            }
        }
    }

    let _ = app_handle.emit("pipeline-event", PipelineEvent::StateChange {
        state: "stopped".to_string(),
    });

    Ok(())
}

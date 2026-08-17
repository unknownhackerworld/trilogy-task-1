use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::mpsc;
use std::thread;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};

use super::{AsrEngine, AsrEvent};

enum AudioMsg {
    Samples(Vec<i16>),
    Flush,
    LanguageChange(String),
    Stop,
}

pub struct DeepgramAsr {
    audio_tx: mpsc::SyncSender<AudioMsg>,
    event_rx: mpsc::Receiver<AsrEvent>,
    language: String,
    // Kept alive to prevent the worker thread from being dropped
    _worker: thread::JoinHandle<()>,
}

impl DeepgramAsr {
    pub fn new(api_key: &str, language: &str) -> Result<Self> {
        let (audio_tx, audio_rx) = mpsc::sync_channel::<AudioMsg>(256);
        let (event_tx, event_rx) = mpsc::channel::<AsrEvent>();

        let api_key = api_key.to_string();
        let lang_for_thread = language.to_string();

        // Capture preview before api_key is moved into the thread closure
        let key_preview = format!(
            "{}...{}",
            &api_key[..4.min(api_key.len())],
            &api_key[api_key.len().saturating_sub(4)..]
        );

        let worker = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(deepgram_worker(api_key, lang_for_thread, audio_rx, event_tx));
        });

        println!("[Deepgram] ASR engine created — language: {}, key: {}", language, key_preview);

        Ok(Self {
            audio_tx,
            event_rx,
            language: language.to_string(),
            _worker: worker,
        })
    }
}

impl AsrEngine for DeepgramAsr {
    fn feed_audio(&mut self, samples: &[i16]) -> Result<Vec<AsrEvent>> {
        // Non-blocking send — drop chunk if buffer full (avoids backpressure in pipeline)
        let _ = self.audio_tx.try_send(AudioMsg::Samples(samples.to_vec()));

        // Drain all available events without blocking
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        Ok(events)
    }

    fn flush(&mut self) -> Result<Vec<AsrEvent>> {
        let _ = self.audio_tx.try_send(AudioMsg::Flush);

        // Give Deepgram time to finalize remaining speech
        std::thread::sleep(std::time::Duration::from_millis(600));

        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        Ok(events)
    }

    fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
        let _ = self
            .audio_tx
            .try_send(AudioMsg::LanguageChange(language.to_string()));
    }
}

impl Drop for DeepgramAsr {
    fn drop(&mut self) {
        let _ = self.audio_tx.try_send(AudioMsg::Stop);
    }
}

// --- Async worker -----------------------------------------------------------

enum SessionResult {
    Stop,
    Reconnect { new_language: String },
    Error(anyhow::Error),
}

async fn deepgram_worker(
    api_key: String,
    initial_language: String,
    audio_rx: mpsc::Receiver<AudioMsg>,
    event_tx: mpsc::Sender<AsrEvent>,
) {
    println!("[Deepgram] Worker started — language: {}", initial_language);

    if api_key.is_empty() {
        println!("[Deepgram] ERROR: API key is empty! Set DEEPGRAM_API_KEY env var or enter it in Settings.");
        let _ = event_tx.send(AsrEvent::Error {
            message: "Deepgram API key is missing. Open Settings (⚙) and enter your key.".into(),
        });
        return;
    }

    let mut language = initial_language;

    loop {
        println!("[Deepgram] Connecting to Deepgram...");
        match run_session(&api_key, &language, &audio_rx, &event_tx).await {
            SessionResult::Stop => {
                println!("[Deepgram] Worker stopped cleanly.");
                break;
            }
            SessionResult::Reconnect { new_language } => {
                println!("[Deepgram] Reconnecting with new language: {}", new_language);
                language = new_language;
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            SessionResult::Error(e) => {
                println!("[Deepgram] Session error: {}. Retrying in 3s...", e);
                let _ = event_tx.send(AsrEvent::Error {
                    message: format!("Deepgram error: {e}"),
                });
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
        }
    }
}

async fn run_session(
    api_key: &str,
    language: &str,
    audio_rx: &mpsc::Receiver<AudioMsg>,
    event_tx: &mpsc::Sender<AsrEvent>,
) -> SessionResult {
    let bcp47 = to_bcp47(language);

    // nova-2-general has the best multilingual support including Tamil
    let url = format!(
        "wss://api.deepgram.com/v1/listen\
         ?model=nova-2-general\
         &language={bcp47}\
         &punctuate=true\
         &interim_results=true\
         &endpointing=700\
         &smart_format=true\
         &encoding=linear16\
         &sample_rate=16000\
         &channels=1"
    );

    let mut request = match url.as_str().into_client_request() {
        Ok(r) => r,
        Err(e) => return SessionResult::Error(anyhow::anyhow!("Bad URL: {}", e)),
    };

    request.headers_mut().insert(
        "Authorization",
        match format!("Token {api_key}").parse() {
            Ok(v) => v,
            Err(e) => return SessionResult::Error(anyhow::anyhow!("Bad API key header: {}", e)),
        },
    );

    let ws_stream = match connect_async(request).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            println!("[Deepgram] ERROR: WebSocket connect failed: {}", e);
            return SessionResult::Error(anyhow::anyhow!("WebSocket connect failed: {}", e));
        }
    };

    println!("[Deepgram] Connected — language: {}, model: nova-2-general", bcp47);

    let (mut ws_sink, mut ws_rx) = ws_stream.split();

    // Spawn reader task — receives transcript JSON from Deepgram
    let event_tx_clone = event_tx.clone();
    let read_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Print raw JSON for debugging
                    println!("[Deepgram] Raw response: {}", &text[..text.len().min(300)]);

                    if let Ok(response) = serde_json::from_str::<DeepgramResponse>(&text) {
                        if let Some(channel) = response.channel {
                            if let Some(alt) = channel.alternatives.into_iter().next() {
                                let transcript = alt.transcript.trim().to_string();
                                if !transcript.is_empty() {
                                    if response.is_final {
                                        println!("[Deepgram] FINAL: {}", transcript);
                                        let _ = event_tx_clone.send(AsrEvent::Final { text: transcript });
                                    } else {
                                        println!("[Deepgram] interim: {}", transcript);
                                        let _ = event_tx_clone.send(AsrEvent::Interim { text: transcript });
                                    }
                                }
                            }
                        }
                    } else {
                        println!("[Deepgram] (non-transcript message, skipped)");
                    }
                }
                Ok(Message::Close(frame)) => {
                    println!("[Deepgram] Connection closed by server: {:?}", frame);
                    break;
                }
                Err(e) => {
                    println!("[Deepgram] ERROR reading from WebSocket: {}", e);
                    break;
                }
                _ => {}
            }
        }
        println!("[Deepgram] Reader task exited");
    });

    // Send loop — reads from the sync mpsc channel and sends audio to Deepgram
    let mut chunks_sent: u64 = 0;
    loop {
        match audio_rx.try_recv() {
            Ok(AudioMsg::Samples(samples)) => {
                // i16 PCM → raw bytes (little-endian, as Deepgram expects)
                let bytes: Vec<u8> = samples
                    .iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect();

                if let Err(e) = ws_sink.send(Message::Binary(bytes)).await {
                    println!("[Deepgram] ERROR sending audio chunk: {}", e);
                    read_task.abort();
                    return SessionResult::Error(anyhow::anyhow!(e));
                }

                chunks_sent += 1;
                // Print every 50 chunks (~5 seconds of audio at 100ms chunks)
                if chunks_sent % 50 == 0 {
                    println!("[Deepgram] Streaming audio... ({} chunks sent)", chunks_sent);
                }
            }

            Ok(AudioMsg::LanguageChange(new_lang)) => {
                // Close this session — outer loop will reconnect with new language
                let _ = ws_sink.send(Message::Close(None)).await;
                read_task.abort();
                return SessionResult::Reconnect { new_language: new_lang };
            }

            Ok(AudioMsg::Flush) => {
                // Tell Deepgram to finalize current utterance
                let _ = ws_sink
                    .send(Message::Text(r#"{"type":"CloseStream"}"#.into()))
                    .await;
                // Wait for read_task to drain remaining responses
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                read_task.abort();
                return SessionResult::Stop;
            }

            Ok(AudioMsg::Stop) => {
                let _ = ws_sink.send(Message::Close(None)).await;
                read_task.abort();
                return SessionResult::Stop;
            }

            Err(mpsc::TryRecvError::Empty) => {
                // No audio right now — yield briefly to avoid busy-spin
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            Err(mpsc::TryRecvError::Disconnected) => {
                let _ = ws_sink.send(Message::Close(None)).await;
                read_task.abort();
                return SessionResult::Stop;
            }
        }
    }
}

// --- Deepgram response types ------------------------------------------------

#[derive(Deserialize)]
struct DeepgramResponse {
    channel: Option<DeepgramChannel>,
    is_final: bool,
}

#[derive(Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}

// --- Language code mapping --------------------------------------------------

fn to_bcp47(lang: &str) -> &str {
    match lang {
        "en" => "en-US",
        "ta" => "ta",
        "hi" => "hi",
        "es" => "es",
        "fr" => "fr",
        "de" => "de",
        "ja" => "ja",
        "zh-CN" | "zh" => "zh-CN",
        "ko" => "ko",
        "ar" => "ar",
        "pt" => "pt-BR",
        "ru" => "ru",
        "it" => "it",
        "nl" => "nl",
        "tr" => "tr",
        "vi" => "vi",
        "th" => "th",
        "id" => "id",
        other => other,
    }
}

use anyhow::{anyhow, Result};
use async_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

use super::resampler::{audio_level, Resampler};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    /// For apps: the process ID as string. For system: the endpoint device ID.
    pub id: String,
    /// Display name: "Zoom Meetings", "Brave - YouTube", "Spotify", etc.
    pub name: String,
    /// Process executable name (e.g. "zoom.exe", "brave.exe")
    pub process_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub is_active: bool,
}

/// Audio chunk sent downstream to ASR.
pub struct AudioChunk {
    pub data: Vec<i16>,
    pub level: f32,
}

pub struct AudioCapture {
    running: Arc<AtomicBool>,
    sender: Sender<AudioChunk>,
    receiver: Receiver<AudioChunk>,
    capture_thread: Option<thread::JoinHandle<()>>,
}

impl AudioCapture {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(300);
        Self {
            running: Arc::new(AtomicBool::new(false)),
            sender,
            receiver,
            capture_thread: None,
        }
    }

    pub fn receiver(&self) -> Receiver<AudioChunk> {
        self.receiver.clone()
    }

    /// List all applications currently producing audio.
    /// Enumerates audio sessions from the default render endpoint,
    /// matching what Windows shows in Settings > Sound > Volume Mixer.
    /// Runs on a dedicated thread to ensure clean COM apartment state.
    #[cfg(windows)]
    pub fn list_devices() -> Result<Vec<AudioDevice>> {
        // Spawn a dedicated thread for COM operations to avoid apartment conflicts
        // with Tauri's runtime threads
        let handle = thread::spawn(|| -> Result<Vec<AudioDevice>> {
            enumerate_audio_sessions()
        });

        handle
            .join()
            .map_err(|_| anyhow!("Audio enumeration thread panicked"))?
    }

    #[cfg(not(windows))]
    pub fn list_devices() -> Result<Vec<AudioDevice>> {
        Ok(vec![])
    }

    /// Start WASAPI loopback capture on the default render endpoint.
    /// Captures all audio going to speakers — includes the selected app
    /// plus any other audio. True per-app capture requires process loopback
    /// (Windows 10 2004+), which will be added in a future update.
    #[cfg(windows)]
    pub fn start(&mut self, device_id: &str) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            self.stop();
        }

        let running = self.running.clone();
        let sender = self.sender.clone();
        let device_id = device_id.to_string();

        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            if let Err(e) = capture_loop_wasapi(&device_id, &running, &sender) {
                error!("Audio capture error: {}", e);
            }
            running.store(false, Ordering::SeqCst);
            info!("Audio capture thread exited");
        });

        self.capture_thread = Some(handle);
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn start(&mut self, _device_id: &str) -> Result<()> {
        Err(anyhow!("Audio capture not supported on this platform"))
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Enumerate audio sessions on a fresh COM-initialized thread.
/// This lists apps that have registered audio sessions with Windows.
#[cfg(windows)]
fn enumerate_audio_sessions() -> Result<Vec<AudioDevice>> {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        // Initialize COM — ignore error if already initialized on this thread
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        // Use eMultimedia role (matches Windows Volume Mixer behavior)
        let default_device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let endpoint_id = default_device.GetId()?.to_string()?;

        // Get device format info
        let audio_client: IAudioClient = default_device.Activate(CLSCTX_ALL, None)?;
        let format_ptr = audio_client.GetMixFormat()?;
        let format = &*format_ptr;
        let sample_rate = format.nSamplesPerSec;
        let channels = format.nChannels;
        CoTaskMemFree(Some(format_ptr as *const _ as *const _));
        drop(audio_client);

        // Get session manager to enumerate apps
        let session_manager: IAudioSessionManager2 =
            default_device.Activate(CLSCTX_ALL, None)?;

        let session_enumerator = session_manager.GetSessionEnumerator()?;
        let session_count = session_enumerator.GetCount()?;

        let mut devices = Vec::new();

        for i in 0..session_count {
            let session: IAudioSessionControl = match session_enumerator.GetSession(i) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Cast to IAudioSessionControl2 for process info
            let session2: IAudioSessionControl2 = match session.cast() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Get process ID — skip PID 0 (system sounds)
            let pid = match session2.GetProcessId() {
                Ok(p) if p != 0 => p,
                _ => continue,
            };

            // Get session state — include Active and Inactive, skip Expired
            let state = session.GetState().unwrap_or(AudioSessionStateExpired);
            if state == AudioSessionStateExpired {
                continue;
            }

            // Get process name from PID
            let process_name = get_process_name(pid).unwrap_or_else(|| format!("pid-{}", pid));

            // Get display name from session (often empty — apps rarely set it)
            let display_name = session
                .GetDisplayName()
                .ok()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_default();

            // Build a user-friendly display name
            let name = if !display_name.is_empty()
                && display_name != ""
                && !display_name.starts_with('@')
                && !display_name.starts_with('{')
            {
                display_name
            } else {
                // Use process name without .exe, capitalized
                let clean = process_name.trim_end_matches(".exe").to_string();
                capitalize_first(&clean)
            };

            devices.push(AudioDevice {
                id: endpoint_id.clone(),
                name,
                process_name,
                sample_rate,
                channels,
                is_active: state == AudioSessionStateActive,
            });
        }

        // Sort: active (producing sound now) first, then alphabetical
        devices.sort_by(|a, b| b.is_active.cmp(&a.is_active).then(a.name.cmp(&b.name)));

        // Deduplicate by process name (some apps create multiple sessions)
        devices.dedup_by(|a, b| a.process_name == b.process_name);

        CoUninitialize();
        Ok(devices)
    }
}

/// Get process executable name from a PID using ToolHelp32 snapshot.
#[cfg(windows)]
fn get_process_name(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Diagnostics::ToolHelp::*;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;

        let mut entry = PROCESSENTRY32W::default();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let name_len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
                    let _ = CloseHandle(snapshot);
                    return Some(name);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        None
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// WASAPI loopback capture loop.
#[cfg(windows)]
fn capture_loop_wasapi(
    device_id: &str,
    running: &Arc<AtomicBool>,
    sender: &Sender<AudioChunk>,
) -> Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok();

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let device_id_wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        let device = enumerator.GetDevice(PCWSTR(device_id_wide.as_ptr()))?;

        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let format_ptr = audio_client.GetMixFormat()?;
        let format = &*format_ptr;

        let native_rate = format.nSamplesPerSec;
        let native_channels = format.nChannels;
        // WASAPI shared mode returns float32 on virtually all modern Windows devices.
        // Check wBitsPerSample: 32 = float32, 16 = int16.
        let is_float32 = format.wBitsPerSample == 32;

        println!(
            "[Audio] WASAPI loopback: {}Hz {} ch {} — format: {}",
            native_rate,
            native_channels,
            if is_float32 { "float32" } else { "int16" },
            format.wBitsPerSample
        );

        // Initialize in shared loopback mode
        let buffer_duration = 10_000_000i64; // 1 second in 100ns units
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            buffer_duration,
            0,
            format_ptr,
            None,
        )?;

        let capture_client: IAudioCaptureClient = audio_client.GetService()?;

        audio_client.Start()?;

        let mut accumulated = Vec::<i16>::new();
        let target_chunk_samples = 1600; // 100ms at 16kHz

        while running.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(20));

            loop {
                let packet_size = match capture_client.GetNextPacketSize() {
                    Ok(s) => s,
                    Err(_) => break,
                };

                if packet_size == 0 {
                    break;
                }

                let mut buffer_ptr = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                if capture_client
                    .GetBuffer(&mut buffer_ptr, &mut num_frames, &mut flags, None, None)
                    .is_err()
                {
                    break;
                }

                if num_frames > 0 && !buffer_ptr.is_null() {
                    let sample_count = num_frames as usize * native_channels as usize;

                    if flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 {
                        let out_samples = (num_frames as usize * 16000) / native_rate as usize;
                        accumulated.extend(std::iter::repeat(0i16).take(out_samples));
                    } else if is_float32 {
                        // float32 PCM — convert to i16 first, then downsample
                        let float_slice = std::slice::from_raw_parts(
                            buffer_ptr as *const f32,
                            sample_count,
                        );
                        let as_i16: Vec<i16> = float_slice
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                            .collect();
                        let mono = Resampler::downsample_simple(
                            &as_i16,
                            native_rate,
                            native_channels,
                        );
                        accumulated.extend_from_slice(&mono);
                    } else {
                        // int16 PCM — downsample directly
                        let buffer_slice = std::slice::from_raw_parts(
                            buffer_ptr as *const i16,
                            sample_count,
                        );
                        let mono = Resampler::downsample_simple(
                            buffer_slice,
                            native_rate,
                            native_channels,
                        );
                        accumulated.extend_from_slice(&mono);
                    }
                }

                let _ = capture_client.ReleaseBuffer(num_frames);
            }

            // Emit chunks in 100ms segments
            while accumulated.len() >= target_chunk_samples {
                let chunk_data: Vec<i16> = accumulated.drain(..target_chunk_samples).collect();
                let level = audio_level(&chunk_data);

                if sender.try_send(AudioChunk { data: chunk_data, level }).is_err() {
                    warn!("Audio queue full, dropping chunk");
                }
            }
        }

        audio_client.Stop()?;
        CoTaskMemFree(Some(format_ptr as *const _ as *const _));
    }

    Ok(())
}

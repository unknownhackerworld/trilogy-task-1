# Speech Translator

A real-time speech translator desktop application built with Tauri (Rust + Svelte). Captures audio from any running application — Zoom, Google Meet, Teams, or any other — transcribes it using Deepgram or local Whisper, and translates it in real-time using MyMemory (free) or Google Cloud Translation.

Translated text appears as an overlay on your screen, invisible to screen share participants.

---

## Features

- **Real-time transcription** — Deepgram Nova-2 streams words as they are spoken (~300ms latency)
- **Live translation** — translates each completed sentence to your target language
- **20+ language pairs** — English, Tamil, Hindi, Spanish, French, German, Japanese, Chinese, Korean, Arabic, and more
- **Silence detection and punctuation** — Deepgram automatically detects sentence boundaries and adds punctuation
- **Overlay mode** — a transparent always-on-top subtitle bar that floats over your meeting window
- **Hidden from screen share** — the overlay is invisible to Zoom, Meet, OBS, and all screen capture tools
- **Per-app audio capture** — select which running application to capture audio from
- **Dual ASR engines** — Deepgram (cloud, real-time) or Whisper (local, offline, free)
- **Dual translation engines** — MyMemory API (free) or Google Cloud Translation (paid, best quality)
- **Translation memory cache** — repeated phrases (e.g. "can you hear me?") are cached and returned instantly
- **Settings UI** — configure API keys and engines without touching any files

---

## Screenshots

```
Control Panel                         Overlay Mode
┌──────────────────────────────────┐  ┌──────────────────────────────────────┐
│ Speech Translator              ⚙ │  │ ● ══════════════════════  ▤  ◁  ×   │
│                                  │  │  நாம் இலக்கை 15 சதவீதம்            │
│ [Zoom Meeting          ▼  ⟳]    │  │  தாண்டினோம்.                         │
│                                  │  └──────────────────────────────────────┘
│ [English ▼]  ──►  [Tamil ▼]    │
│                                  │
│ [● Start Translating           ] │
│                                  │
│  ORIGINAL        TRANSLATION     │
│ ─────────────────────────────── │
│  We exceeded  │  நாம் இலக்கை   │
│  our target   │  15 சதவீதம்    │
│  by 15%       │  தாண்டினோம்.   │
│               │                  │
│  I think...   │  ...             │
│  (interim)    │                  │
│ ──────────────────────────────── │
│  ⏱ 04:23    ◆ Pop Out           │
└──────────────────────────────────┘
```

---

## Requirements

| Requirement | Minimum Version |
|---|---|
| Windows | Windows 10 version 2004 (build 19041) or later |
| Node.js | 18+ |
| Rust | 1.70+ (via rustup) |
| Visual Studio Build Tools | Desktop development with C++ workload |
| CMake | Required by whisper-rs (even if using Deepgram) |
| NVIDIA GPU (optional) | CUDA for faster local Whisper inference |

> macOS and Linux support is planned. The audio capture layer is platform-abstracted; Windows is the current implementation.

---

## Installation

### 1. Clone and install dependencies

```bash
git clone <repo-url>
cd speech-translator
npm install
npm install -D @tauri-apps/cli
```

### 2. Install Rust

```bash
# Install rustup from https://rustup.rs
rustup update
```

### 3. Install CMake

```bash
winget install Kitware.CMake
```

### 4. Set up environment

```bash
cp .env.example .env
```

Open `.env` and fill in your keys (see [Configuration](#configuration) below).

### 5. Run in development mode

```powershell
# Set your Deepgram API key for the session
$env:DEEPGRAM_API_KEY="your_key_here"

npx tauri dev
```

First run compiles the Rust backend — this takes 3–5 minutes. Subsequent runs are fast.

### 6. Build for production

```bash
npx tauri build
```

Installer is created at `src-tauri/target/release/bundle/`.

---

## Configuration

### Environment Variables

Set these in your `.env` file or in the shell before running:

| Variable | Description |
|---|---|
| `DEEPGRAM_API_KEY` | Deepgram API key. Overrides any value saved in the app settings. |
| `GOOGLE_TRANSLATE_API_KEY` | Google Cloud Translation API key (optional, for paid translation). |

### In-App Settings (⚙ gear icon)

Click the gear icon in the top-right of the control panel to open Settings:

- **ASR Engine** — switch between Deepgram (cloud) and Whisper (local)
- **Deepgram API Key** — enter and save your key; persisted to local config file
- **Translation Engine** — switch between MyMemory (free) and Google Translate (paid)

Settings are saved to `%APPDATA%\speech-translator\settings.json`.

---

## API Keys

### Deepgram (Speech-to-Text)

1. Sign up at [console.deepgram.com](https://console.deepgram.com) — no credit card required
2. You get **$200 free credit** on signup (~700 hours of transcription)
3. Create an API key in the dashboard
4. Paste it into the Settings panel or set `DEEPGRAM_API_KEY` in your environment

### Google Cloud Translation (optional)

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Enable the **Cloud Translation API**
3. Create an API key under APIs & Services > Credentials
4. Set it in Settings or in `GOOGLE_TRANSLATE_API_KEY`

If you do not set a Google key, the app uses **MyMemory API** (free, ~500 requests/day, no key needed).

---

## How It Works

### Pipeline

```
Selected App (Zoom, Meet, etc.)
        │
        ▼
WASAPI Audio Loopback Capture
(captures render stream of the target process)
        │
        ▼
Format Conversion
(float32 → int16, stereo → mono, 48kHz → 16kHz)
        │
        ▼
ASR Engine
├── Deepgram Nova-2 (WebSocket streaming, ~300ms latency)
│   └── returns interim + final results with punctuation
└── Whisper large-v3-turbo (local GPU, ~1-2s latency)
    └── silence-based chunking, beam search decoding
        │
        ▼
Translation Engine
├── MyMemory API (free, REST call on final sentences)
└── Google Cloud Translation v2 (paid, higher quality)
        │
        ▼
Tauri IPC Events → Svelte Frontend
        │
        ▼
Transcript Panel (control panel) + Overlay Window
```

### Audio Capture Details

- Uses **WASAPI loopback** on the default render endpoint
- Enumerates active audio sessions via `IAudioSessionManager2`
- Captures at the device's native format (typically float32, 48kHz, stereo)
- Converts to 16kHz mono int16 PCM before sending to ASR
- Only sends non-silent audio chunks (circuit breaker prevents streaming silence)

### Overlay Window

- Transparent, always-on-top window using Tauri's window API
- Uses `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` — the overlay is visible on your physical display but **invisible** to all screen capture APIs (Zoom share, OBS, screenshots)
- Drag to reposition; resize by dragging the edges
- Double-click the title bar to snap back to bottom-center

---

## Project Structure

```
speech-translator/
├── src/                         # Svelte frontend
│   ├── App.svelte               # Root component, routing (panel vs overlay)
│   ├── pages/
│   │   ├── ControlPanel.svelte  # Main window UI
│   │   └── OverlayMode.svelte   # Floating subtitle bar
│   ├── lib/
│   │   ├── components/
│   │   │   ├── AppPicker.svelte      # Audio source selector
│   │   │   ├── LanguageSelector.svelte
│   │   │   ├── TranscriptPanel.svelte
│   │   │   ├── StatusBar.svelte
│   │   │   └── SettingsPanel.svelte  # API key + engine config
│   │   ├── stores/
│   │   │   ├── pipeline.ts      # Pipeline state, transcript, audio level
│   │   │   └── settings.ts      # App settings store
│   │   └── types.ts             # TypeScript interfaces
│   └── styles/
│       └── global.css           # CSS variables, dark theme
│
├── src-tauri/                   # Rust backend
│   └── src/
│       ├── main.rs              # Entry point
│       ├── lib.rs               # Tauri app setup, command registration
│       ├── commands.rs          # IPC command handlers (called from frontend)
│       ├── state.rs             # AppState, AppSettings, settings persistence
│       ├── pipeline.rs          # Audio → ASR → Translation → UI event loop
│       ├── audio/
│       │   ├── capture.rs       # WASAPI loopback capture, session enumeration
│       │   └── resampler.rs     # Float32→int16 conversion, 48kHz→16kHz downsample
│       ├── asr/
│       │   ├── mod.rs           # AsrEngine trait, factory function
│       │   ├── deepgram.rs      # Deepgram Nova-2 WebSocket streaming client
│       │   └── whisper.rs       # Local Whisper via whisper-rs (GPU accelerated)
│       └── translation/
│           ├── mod.rs           # TranslationBackend trait, cache, factory
│           ├── libre.rs         # MyMemory API (free)
│           └── google.rs        # Google Cloud Translation v2 (paid)
│
├── .env.example                 # Environment variable template
├── tauri.conf.json              # Window config, CSP, bundle settings
└── README.md
```

---

## Supported Languages

| Language | Code | Deepgram ASR | Translation |
|---|---|---|---|
| English | `en` | Nova-2 (best) | Yes |
| Tamil | `ta` | Nova-2 | Yes |
| Hindi | `hi` | Nova-2 | Yes |
| Telugu | `te` | Nova-2 | Yes |
| Bengali | `bn` | Nova-2 | Yes |
| Spanish | `es` | Nova-2 | Yes |
| French | `fr` | Nova-2 | Yes |
| German | `de` | Nova-2 | Yes |
| Japanese | `ja` | Nova-2 | Yes |
| Korean | `ko` | Nova-2 | Yes |
| Chinese (Mandarin) | `zh-CN` | Nova-2 | Yes |
| Arabic | `ar` | Nova-2 | Yes |
| Portuguese | `pt` | Nova-2 | Yes |
| Russian | `ru` | Nova-2 | Yes |
| Italian | `it` | Nova-2 | Yes |
| Dutch | `nl` | Nova-2 | Yes |
| Turkish | `tr` | Nova-2 | Yes |
| Vietnamese | `vi` | Nova-2 | Yes |
| Thai | `th` | Nova-2 | Yes |
| Indonesian | `id` | Nova-2 | Yes |

---

## Whisper Model Setup (Local ASR)

If using the Whisper engine instead of Deepgram, download the model file first:

```powershell
# Create models directory
mkdir "$env:LOCALAPPDATA\speech-translator\models"

# Download large-v3-turbo (recommended — fast GPU inference, good accuracy)
curl -L -o "$env:LOCALAPPDATA\speech-translator\models\ggml-large-v3-turbo.bin" `
  "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"
```

Model sizes:

| Model | Size | VRAM | Speed | Accuracy |
|---|---|---|---|---|
| `base` | 145 MB | 1 GB | Fast | Decent |
| `small` | 466 MB | 2 GB | Good | Good |
| `large-v3-turbo` | 1.6 GB | 6 GB | Fast | Best (recommended) |
| `large-v3` | 3.1 GB | 10 GB | Slow | Best |

GPU acceleration uses CUDA automatically if an NVIDIA GPU is present.

---

## Latency Targets

| Stage | Deepgram | Whisper (GPU) |
|---|---|---|
| Audio capture | ~20ms | ~20ms |
| Speech-to-text | ~300ms streaming | ~800ms per chunk |
| Translation (MyMemory) | ~300ms | ~300ms |
| Translation (Google) | ~100ms | ~100ms |
| **Total (end of utterance)** | **~600–900ms** | **~1.1–1.5s** |

---

## Security

- **API keys never reach the frontend** — all keys are loaded in the Rust backend only; the Svelte WebView has no access to credentials
- **IPC command allowlist** — only explicitly registered Tauri commands are callable from the frontend
- **All rendered text uses `textContent`** — transcript text is never rendered as HTML, preventing XSS from crafted speech input
- **Input validation** — language codes and settings values are validated against whitelists in the Rust backend before any operation
- **Overlay hidden from screen capture** — `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` prevents the overlay from appearing in any screen share or recording
- **Audio never crosses IPC** — raw audio bytes stay in the Rust backend; the frontend only receives text events

---

## Troubleshooting

### No text appearing in the transcript panel

1. Check the status bar at the bottom for a red error message
2. Run with `RUST_LOG=debug npx tauri dev` and check the terminal for `[Deepgram]` log lines
3. Confirm `[Deepgram] Connected` appears — if not, check your API key
4. Confirm `[Audio] WASAPI loopback: 48000Hz 2 ch float32` appears — if not, audio capture failed

### "API key is missing" error

```powershell
$env:DEEPGRAM_API_KEY="your_key_here"
npx tauri dev
```

Or open **Settings (⚙)** in the app, enter your key, and click Save.

### Old settings still loading Whisper

Delete the saved settings file to reset to defaults:

```powershell
del "$env:APPDATA\speech-translator\settings.json"
```

### App crashes on startup (exit code 0xcfffffff)

This is a Whisper model load failure. Either:
- Download the model file (see [Whisper Model Setup](#whisper-model-setup-local-asr))
- Or switch to Deepgram: set `asr_engine = "deepgram"` in settings

### Deepgram sends audio but returns empty transcripts

The audio format conversion may have failed. Check the terminal for:
```
[Audio] WASAPI loopback: 48000Hz 2 ch float32
```
If it shows `int16` but the actual device is float32, file an issue.

---

## Known Limitations

- **Windows only** — macOS (ScreenCaptureKit) and Linux (PipeWire) support is planned
- **Overlay not hidden on X11 Linux** — `WDA_EXCLUDEFROMCAPTURE` is Windows-only; there is no reliable equivalent on X11
- **Chrome multi-tab** — WASAPI loopback captures all Chrome audio; you cannot isolate a single tab from the desktop app
- **Whisper has no interim results** — only final sentences appear; no live word-by-word display

---

## License

MIT

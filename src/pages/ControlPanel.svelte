<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import AppPicker from "../lib/components/AppPicker.svelte";
  import LanguageSelector from "../lib/components/LanguageSelector.svelte";
  import TranscriptPanel from "../lib/components/TranscriptPanel.svelte";
  import StatusBar from "../lib/components/StatusBar.svelte";
  import SettingsPanel from "../lib/components/SettingsPanel.svelte";
  import WaveformVisualizer from "../lib/components/WaveformVisualizer.svelte";
  import {
    pipelineState,
    selectedDevice,
    durationSecs,
    clearTranscript,
  } from "../lib/stores/pipeline";
  import { sourceLang, targetLang } from "../lib/stores/settings";

  let starting = $state(false);

  async function startTranslation() {
    if (!$selectedDevice) return;
    starting = true;

    try {
      await invoke("set_languages", {
        sourceLang: $sourceLang,
        targetLang: $targetLang,
      });

      await invoke("start_translation", {
        deviceId: $selectedDevice.id,
        deviceName: $selectedDevice.name,
        sampleRate: $selectedDevice.sample_rate,
        channels: $selectedDevice.channels,
      });

      pipelineState.set("running");
      durationSecs.set(0);
    } catch (e: any) {
      console.error("Failed to start:", e);
      pipelineState.set("error");
    } finally {
      starting = false;
    }
  }

  async function stopTranslation() {
    try {
      await invoke("stop_translation");
    } catch (e) {
      console.error("Failed to stop:", e);
    }
    pipelineState.set("idle");
  }

  function handleToggle() {
    if ($pipelineState === "running") {
      stopTranslation();
    } else {
      startTranslation();
    }
  }

  // Expose stop for overlay close
  (window as any).__stopFromOverlay = stopTranslation;
</script>

<div class="control-panel">
  <div class="panel-top-bar">
    <span class="app-title">Speech Translator</span>
    <SettingsPanel />
  </div>

  <div class="controls">
    <AppPicker />

    <LanguageSelector disabled={$pipelineState === "running"} />

    <button
      class="start-btn"
      class:running={$pipelineState === "running"}
      class:disabled={!$selectedDevice || starting}
      onclick={handleToggle}
      disabled={!$selectedDevice && $pipelineState !== "running"}
    >
      {#if $pipelineState === "running"}
        <span class="dot recording"></span>
        <span>Translating... </span>
        <span class="stop-label">■ Stop</span>
      {:else if starting}
        <span>Starting...</span>
      {:else}
        <span class="dot"></span>
        <span>Start Translating</span>
      {/if}
    </button>
  </div>

  <WaveformVisualizer />

  <div class="divider"></div>

  <TranscriptPanel />

  <StatusBar />
</div>

<style>
  .control-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--padding);
    gap: 14px;
  }

  .panel-top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }

  .app-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.3px;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex-shrink: 0;
  }

  .divider {
    height: 1px;
    background: var(--border);
    flex-shrink: 0;
  }

  .start-btn {
    width: 100%;
    height: 50px;
    background: var(--accent-blue);
    color: white;
    border-radius: var(--radius);
    font-size: 15px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.2s;
  }

  .start-btn:hover:not(:disabled) {
    background: var(--accent-blue-hover);
  }

  .start-btn.running {
    background: var(--success-green);
  }

  .start-btn.running:hover {
    background: var(--error-red);
  }

  .start-btn.running:hover .stop-label {
    display: inline;
  }

  .start-btn.running:hover span:not(.stop-label):not(.dot) {
    display: none;
  }

  .start-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .stop-label {
    display: none;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: white;
    opacity: 0.8;
  }

  .dot.recording {
    background: var(--error-red);
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>

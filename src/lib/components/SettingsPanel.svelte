<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { settings } from "../stores/settings";
  import type { AppSettings } from "../types";

  let open = $state(false);
  let saving = $state(false);
  let saveError = $state("");
  let saveSuccess = $state(false);
  let showKey = $state(false);

  // Local editable copy
  let local = $state<AppSettings>({ ...$settings });

  // Keep local in sync when settings store updates (e.g. on first load from backend)
  $effect(() => {
    local = { ...$settings };
  });

  function toggle() {
    open = !open;
    saveError = "";
    saveSuccess = false;
  }

  async function save() {
    saving = true;
    saveError = "";
    saveSuccess = false;

    try {
      await invoke("save_settings", { settings: local });
      settings.set({ ...local });
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
        open = false;
      }, 1200);
    } catch (e: any) {
      saveError = e?.message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<!-- Gear button -->
<button class="gear-btn" onclick={toggle} title="Settings">
  ⚙
</button>

{#if open}
  <!-- Backdrop -->
  <div class="backdrop" onclick={toggle}></div>

  <!-- Panel -->
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">Settings</span>
      <button class="close-btn" onclick={toggle}>✕</button>
    </div>

    <div class="panel-body">

      <!-- ASR Engine -->
      <div class="field">
        <label>Speech Recognition (ASR)</label>
        <div class="radio-group">
          <label class="radio-option" class:selected={local.asr_engine === "deepgram"}>
            <input type="radio" bind:group={local.asr_engine} value="deepgram" />
            Deepgram
            <span class="badge cloud">Cloud · Real-time</span>
          </label>
          <label class="radio-option" class:selected={local.asr_engine === "whisper"}>
            <input type="radio" bind:group={local.asr_engine} value="whisper" />
            Whisper
            <span class="badge local">Local · Offline</span>
          </label>
        </div>
      </div>

      <!-- Deepgram API Key (shown only when deepgram selected) -->
      {#if local.asr_engine === "deepgram"}
        <div class="field">
          <label>Deepgram API Key</label>
          <div class="key-input-row">
            <input
              class="key-input"
              type={showKey ? "text" : "password"}
              bind:value={local.deepgram_api_key}
              placeholder="Paste your Deepgram API key..."
              autocomplete="off"
              spellcheck="false"
            />
            <button class="eye-btn" onclick={() => showKey = !showKey}>
              {showKey ? "Hide" : "Show"}
            </button>
          </div>
          <p class="hint">
            Get a free $200 credit at
            <a href="https://console.deepgram.com" target="_blank" rel="noreferrer">console.deepgram.com</a>
            — no credit card needed.
          </p>
        </div>
      {/if}

      <!-- Translation Engine -->
      <div class="field">
        <label>Translation Engine</label>
        <div class="radio-group">
          <label class="radio-option" class:selected={local.translation_engine === "libre"}>
            <input type="radio" bind:group={local.translation_engine} value="libre" />
            LibreTranslate
            <span class="badge local">Free</span>
          </label>
          <label class="radio-option" class:selected={local.translation_engine === "google"}>
            <input type="radio" bind:group={local.translation_engine} value="google" />
            Google Translate
            <span class="badge cloud">Paid · Best quality</span>
          </label>
        </div>
      </div>

    </div>

    <div class="panel-footer">
      {#if saveError}
        <span class="error-msg">{saveError}</span>
      {/if}
      {#if saveSuccess}
        <span class="success-msg">Saved! Restart translation to apply.</span>
      {/if}
      <button class="save-btn" onclick={save} disabled={saving}>
        {saving ? "Saving..." : "Save Settings"}
      </button>
    </div>
  </div>
{/if}

<style>
  .gear-btn {
    background: transparent;
    color: var(--text-secondary);
    font-size: 18px;
    padding: 4px 6px;
    border-radius: 6px;
    line-height: 1;
    transition: color 0.15s;
  }

  .gear-btn:hover {
    color: var(--text-primary);
    background: var(--bg-surface);
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
  }

  .panel {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 20;
    width: 420px;
    max-width: calc(100vw - 32px);
    background: #1e1e1e;
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border);
  }

  .panel-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .panel-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field > label {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .radio-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    cursor: pointer;
    font-size: 13px;
    color: var(--text-primary);
    transition: border-color 0.15s;
  }

  .radio-option input[type="radio"] {
    accent-color: var(--accent-blue);
  }

  .radio-option.selected {
    border-color: var(--accent-blue);
    background: rgba(37, 99, 235, 0.08);
  }

  .badge {
    margin-left: auto;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 20px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .badge.cloud {
    background: rgba(74, 222, 128, 0.15);
    color: var(--text-translation);
  }

  .badge.local {
    background: rgba(245, 158, 11, 0.15);
    color: var(--warning-amber);
  }

  .key-input-row {
    display: flex;
    gap: 8px;
  }

  .key-input {
    flex: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: monospace;
    transition: border-color 0.15s;
  }

  .key-input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .eye-btn {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 0 12px;
    white-space: nowrap;
  }

  .eye-btn:hover {
    color: var(--text-primary);
  }

  .hint {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .hint a {
    color: var(--accent-blue);
    text-decoration: none;
  }

  .hint a:hover {
    text-decoration: underline;
  }

  .panel-footer {
    padding: 14px 20px 16px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .save-btn {
    width: 100%;
    height: 42px;
    background: var(--accent-blue);
    color: white;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 600;
    transition: background 0.15s;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--accent-blue-hover);
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-msg {
    font-size: 12px;
    color: var(--error-red);
  }

  .success-msg {
    font-size: 12px;
    color: var(--text-translation);
  }
</style>

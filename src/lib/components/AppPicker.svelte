<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { audioDevices, selectedDevice } from "../stores/pipeline";
  import type { AudioDevice } from "../types";

  let loading = $state(false);
  let error = $state("");

  async function refreshDevices() {
    loading = true;
    error = "";
    try {
      const devices = await invoke<AudioDevice[]>("list_audio_devices");
      audioDevices.set(devices);
      // Auto-select first device if none selected
      if (devices.length > 0 && !$selectedDevice) {
        selectedDevice.set(devices[0]);
      }
    } catch (e: any) {
      error = e?.message || "Failed to list audio devices";
    } finally {
      loading = false;
    }
  }

  function onSelect(event: Event) {
    const target = event.target as HTMLSelectElement;
    const device = $audioDevices.find((d) => d.process_name === target.value);
    if (device) {
      selectedDevice.set(device);
    }
  }

  // Load devices on mount
  $effect(() => {
    refreshDevices();
  });
</script>

<div class="app-picker">
  <div class="picker-row">
    <div class="icon">🔊</div>
    <select
      class="device-select"
      value={$selectedDevice?.process_name || ""}
      onchange={onSelect}
      disabled={loading}
    >
      {#if $audioDevices.length === 0}
        <option value="" disabled>
          {loading ? "Scanning..." : "No audio sources found"}
        </option>
      {:else}
        <option value="" disabled>Select an audio source...</option>
        {#each $audioDevices as device}
          <option value={device.process_name}>
            {device.name}{device.is_active ? " 🔊" : ""} — {device.process_name}
          </option>
        {/each}
      {/if}
    </select>
    <button class="refresh-btn" onclick={refreshDevices} disabled={loading}>
      ⟳
    </button>
  </div>
  {#if error}
    <p class="error-text">{error}</p>
  {/if}
  {#if $audioDevices.length === 0 && !loading}
    <p class="hint-text">
      Start a meeting or play audio, then click ⟳ to detect sources.
    </p>
  {/if}
</div>

<style>
  .app-picker {
    background: var(--bg-surface);
    border-radius: var(--radius);
    padding: 12px;
    border: 1px solid var(--border);
  }

  .picker-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    font-size: 18px;
    width: 24px;
    text-align: center;
  }

  .device-select {
    flex: 1;
    height: 40px;
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0 12px;
    font-size: 14px;
    appearance: none;
    cursor: pointer;
  }

  .device-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-btn {
    width: 40px;
    height: 40px;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 16px;
    transition: all 0.15s;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--border);
    color: var(--text-primary);
  }

  .error-text {
    color: var(--error-red);
    font-size: 12px;
    margin-top: 8px;
    padding-left: 34px;
  }

  .hint-text {
    color: var(--text-secondary);
    font-size: 12px;
    margin-top: 8px;
    padding-left: 34px;
  }
</style>

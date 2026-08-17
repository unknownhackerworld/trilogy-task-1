<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    pipelineState,
    audioLevel,
    errorMessage,
    formattedDuration,
    durationSecs,
  } from "../stores/pipeline";

  let durationInterval: number | null = null;

  $effect(() => {
    if ($pipelineState === "running") {
      durationInterval = window.setInterval(() => {
        durationSecs.update((n) => n + 1);
      }, 1000);
    } else {
      if (durationInterval) {
        clearInterval(durationInterval);
        durationInterval = null;
      }
    }
  });

  async function openOverlay() {
    try {
      await invoke("open_overlay");
    } catch (e) {
      console.error("Failed to open overlay:", e);
    }
  }
</script>

<div class="status-bar" class:has-error={!!$errorMessage}>
  {#if $errorMessage}
    <span class="error-msg">⚠ {$errorMessage}</span>
  {:else if $pipelineState === "running"}
    <span class="stat">
      <span class="level-bar">
        <span class="level-fill" style="width: {$audioLevel * 100}%"></span>
      </span>
    </span>
    <span class="stat">⏱ {$formattedDuration}</span>
  {:else}
    <span class="stat idle">Ready</span>
  {/if}

  <button class="overlay-btn" onclick={openOverlay} title="Pop out as overlay (Ctrl+Shift+O)">
    ◆ Pop Out
  </button>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    border: 1px solid var(--border);
    gap: 16px;
  }

  .status-bar.has-error {
    border-color: var(--error-red);
    background: rgba(239, 68, 68, 0.08);
  }

  .stat {
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .stat.idle {
    color: var(--text-secondary);
  }

  .error-msg {
    flex: 1;
    font-size: 12px;
    color: var(--error-red);
  }

  .level-bar {
    width: 60px;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }

  .level-fill {
    height: 100%;
    background: var(--success-green);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .overlay-btn {
    margin-left: auto;
    height: 28px;
    padding: 0 12px;
    background: var(--bg-surface);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    transition: all 0.15s;
  }

  .overlay-btn:hover {
    background: var(--accent-blue);
    color: var(--text-primary);
    border-color: var(--accent-blue);
  }
</style>

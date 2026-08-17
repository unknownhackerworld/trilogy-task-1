<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { PipelineEvent } from "../lib/types";

  let sourceText = $state("");
  let translationText = $state("Waiting for speech...");
  let interimText = $state("");
  let showSource = $state(true);
  let isRecording = $state(true);
  let dotVisible = $state(true);

  // Dragging state
  let isDragging = $state(false);

  onMount(async () => {
    const unlisten = await listen<PipelineEvent>("pipeline-event", (event) => {
      const payload = event.payload;
      switch (payload.type) {
        case "Interim":
          interimText = payload.text || "";
          break;
        case "Translated":
          sourceText = payload.source_text || "";
          translationText = payload.translated_text || "";
          interimText = "";
          break;
        case "Error":
          isRecording = false;
          break;
        case "StateChange":
          if (payload.state === "stopped") {
            isRecording = false;
          }
          break;
      }
    });

    // Pulse animation
    const pulseInterval = setInterval(() => {
      if (isRecording) {
        dotVisible = !dotVisible;
      }
    }, 800);

    return () => {
      unlisten();
      clearInterval(pulseInterval);
    };
  });

  function toggleSource() {
    showSource = !showSource;
  }

  async function expandToPanel() {
    try {
      await invoke("close_overlay");
    } catch (e) {
      console.error(e);
    }
  }

  async function closeOverlay() {
    try {
      await invoke("stop_translation");
      await invoke("close_overlay");
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="overlay" data-tauri-drag-region>
  <div class="top-bar" data-tauri-drag-region>
    <span class="rec-dot" class:visible={dotVisible} class:error={!isRecording}>●</span>
    <div class="drag-area" data-tauri-drag-region></div>
    <div class="controls">
      <button class="ctrl-btn" onclick={toggleSource} title={showSource ? "Hide source" : "Show source"}>
        {showSource ? "◁" : "▷"}
      </button>
      <button class="ctrl-btn" onclick={expandToPanel} title="Expand to panel">▤</button>
      <button class="ctrl-btn close" onclick={closeOverlay} title="Stop & close">×</button>
    </div>
  </div>

  <div class="text-area">
    {#if showSource && sourceText}
      <p class="source">{sourceText}</p>
    {/if}
    <p class="translation">{translationText}</p>
    {#if interimText}
      <p class="interim">{interimText}</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: rgba(26, 26, 26, 0.92);
    border-radius: 12px;
    border: 1px solid #333;
    overflow: hidden;
    user-select: none;
  }

  .top-bar {
    display: flex;
    align-items: center;
    height: 30px;
    background: #111;
    padding: 0 8px;
    flex-shrink: 0;
    cursor: grab;
  }

  .top-bar:active {
    cursor: grabbing;
  }

  .rec-dot {
    font-size: 14px;
    color: #ef4444;
    width: 24px;
    text-align: center;
    transition: opacity 0.2s;
  }

  .rec-dot:not(.visible) {
    opacity: 0.1;
  }

  .rec-dot.error {
    color: #f59e0b;
    opacity: 1 !important;
  }

  .drag-area {
    flex: 1;
    height: 100%;
  }

  .controls {
    display: flex;
    gap: 2px;
  }

  .ctrl-btn {
    width: 26px;
    height: 22px;
    background: transparent;
    color: #6b7280;
    border-radius: 4px;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
  }

  .ctrl-btn:hover {
    background: #333;
    color: #fff;
  }

  .ctrl-btn.close:hover {
    background: #4a2020;
    color: #ef4444;
  }

  .text-area {
    flex: 1;
    padding: 10px 16px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    overflow: hidden;
  }

  .source {
    font-size: 13px;
    color: #ffffff;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .translation {
    font-size: 16px;
    font-weight: 600;
    color: #4ade80;
    line-height: 1.4;
  }

  .interim {
    font-size: 13px;
    color: #9ca3af;
    font-style: italic;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

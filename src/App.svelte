<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import ControlPanel from "./pages/ControlPanel.svelte";
  import OverlayMode from "./pages/OverlayMode.svelte";
  import { handlePipelineEvent } from "./lib/stores/pipeline";
  import { settings } from "./lib/stores/settings";
  import type { PipelineEvent, AppSettings } from "./lib/types";

  let mode: "panel" | "overlay" = "panel";

  // Determine mode from URL hash
  if (window.location.hash === "#/overlay") {
    mode = "overlay";
  }

  onMount(async () => {
    // Listen for pipeline events from Rust backend
    const unlisten = await listen<PipelineEvent>("pipeline-event", (event) => {
      handlePipelineEvent(event.payload);
    });

    // Load settings from backend
    try {
      const savedSettings = await invoke<AppSettings>("get_settings");
      settings.set(savedSettings);
    } catch (e) {
      console.warn("Failed to load settings, using defaults");
    }

    return () => {
      unlisten();
    };
  });
</script>

{#if mode === "overlay"}
  <OverlayMode />
{:else}
  <ControlPanel />
{/if}

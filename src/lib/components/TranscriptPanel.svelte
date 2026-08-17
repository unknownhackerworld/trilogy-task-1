<script lang="ts">
  import { transcript, interimText } from "../stores/pipeline";
  import { tick } from "svelte";

  let scrollContainer: HTMLDivElement;
  let autoScroll = $state(true);

  // Auto-scroll to bottom when new entries arrive
  $effect(() => {
    if ($transcript.length > 0 && autoScroll) {
      tick().then(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      });
    }
  });

  function onScroll() {
    if (!scrollContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
    autoScroll = scrollHeight - scrollTop - clientHeight < 50;
  }
</script>

<div class="transcript-panel">
  <div class="panel-header">
    <div class="col-header source-header">ORIGINAL</div>
    <div class="col-header translation-header">TRANSLATION</div>
  </div>

  <div class="transcript-scroll" bind:this={scrollContainer} onscroll={onScroll}>
    {#if $transcript.length === 0 && !$interimText}
      <div class="empty-state">
        <p>Translated text will appear here once you start translating.</p>
        <p class="hint">Tip: Press Ctrl+Shift+O to pop out as a floating subtitle bar.</p>
      </div>
    {:else}
      <div class="entries">
        {#each $transcript as entry (entry.id)}
          <div class="entry" class:fade-in={true}>
            <div class="source-col">{entry.source_text}</div>
            <div class="translation-col">{entry.translated_text}</div>
          </div>
        {/each}

        {#if $interimText}
          <div class="entry interim">
            <div class="source-col">{$interimText}</div>
            <div class="translation-col waiting">...</div>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if !autoScroll && $transcript.length > 0}
    <button class="scroll-pill" onclick={() => { autoScroll = true; scrollContainer.scrollTop = scrollContainer.scrollHeight; }}>
      ▼ New messages
    </button>
  {/if}
</div>

<style>
  .transcript-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-radius: var(--radius);
    border: 1px solid var(--border);
    overflow: hidden;
    position: relative;
  }

  .panel-header {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 10px 16px;
    flex-shrink: 0;
  }

  .col-header {
    flex: 1;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .source-header {
    color: var(--text-secondary);
  }

  .translation-header {
    color: var(--text-translation);
  }

  .transcript-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    color: var(--text-secondary);
    gap: 8px;
  }

  .empty-state .hint {
    font-size: 12px;
    opacity: 0.7;
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .entry {
    display: flex;
    gap: 16px;
    animation: fadeIn 0.2s ease-out;
  }

  .entry.interim {
    opacity: 0.6;
  }

  .entry.interim .source-col {
    font-style: italic;
    color: var(--text-interim);
  }

  .source-col {
    flex: 1;
    color: var(--text-primary);
    font-size: 14px;
    line-height: 1.5;
  }

  .translation-col {
    flex: 1;
    color: var(--text-translation);
    font-size: 14px;
    font-weight: 500;
    line-height: 1.5;
  }

  .translation-col.waiting {
    color: var(--text-secondary);
    font-style: italic;
  }

  .scroll-pill {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--accent-blue);
    color: white;
    padding: 6px 16px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .scroll-pill:hover {
    background: var(--accent-blue-hover);
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>

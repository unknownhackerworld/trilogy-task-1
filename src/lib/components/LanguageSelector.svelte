<script lang="ts">
  import { sourceLang, targetLang } from "../stores/settings";
  import { LANGUAGES } from "../types";

  let { disabled = false }: { disabled?: boolean } = $props();

  function swapLanguages() {
    const src = $sourceLang;
    const tgt = $targetLang;
    sourceLang.set(tgt);
    targetLang.set(src);
  }
</script>

<div class="language-selector">
  <select
    class="lang-select"
    value={$sourceLang}
    onchange={(e) => sourceLang.set((e.target as HTMLSelectElement).value)}
    {disabled}
  >
    {#each LANGUAGES as lang}
      <option value={lang.code}>{lang.flag} {lang.name}</option>
    {/each}
  </select>

  <button class="swap-btn" onclick={swapLanguages} {disabled} title="Swap languages">
    ⇄
  </button>

  <select
    class="lang-select"
    value={$targetLang}
    onchange={(e) => targetLang.set((e.target as HTMLSelectElement).value)}
    {disabled}
  >
    {#each LANGUAGES as lang}
      <option value={lang.code}>{lang.flag} {lang.name}</option>
    {/each}
  </select>
</div>

<style>
  .language-selector {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .lang-select {
    flex: 1;
    height: 40px;
    background: var(--bg-surface);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0 12px;
    font-size: 13px;
    appearance: none;
    cursor: pointer;
  }

  .lang-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .swap-btn {
    width: 44px;
    height: 40px;
    background: var(--bg-surface);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 18px;
    transition: all 0.15s;
  }

  .swap-btn:hover:not(:disabled) {
    background: var(--accent-blue);
    color: var(--text-primary);
    border-color: var(--accent-blue);
  }

  .swap-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

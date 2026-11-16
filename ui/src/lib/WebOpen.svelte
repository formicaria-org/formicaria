<script lang="ts">
  // **The web** (`decisions.md` 2026-11-17): *Web* in the views list. An address to go to, or words to
  // search; the page opens inside formicaria on the phone, in a tab elsewhere. The views list opens
  // every feature and search finds notes, so this asks for the web and nothing else.
  import { onMount } from 'svelte';

  let {
    onweb,
    onclose,
  }: {
    /** `web:<url>` to go to, or `search:<words>`. */
    onweb: (at: string) => void;
    onclose: () => void;
  } = $props();

  let q = $state('');
  let field = $state<HTMLInputElement>();

  /** What was typed, as a web address, if it reads as one. */
  const address = $derived.by(() => {
    const t = q.trim();
    if (!t || /\s/.test(t)) return null;
    if (/^https?:\/\//i.test(t)) return t;
    return /^[\w-]+(\.[\w-]+)+(:\d+)?(\/.*)?$/.test(t) ? `https://${t}` : null;
  });

  function enter() {
    if (address) return onweb(`web:${address}`);
    if (q.trim()) onweb(`search:${q.trim()}`);
  }
  onMount(() => field?.focus());
</script>

<div class="web-open" role="dialog" aria-label="the web">
  <input
    bind:this={field}
    bind:value={q}
    onkeydown={(e) => {
      if (e.key === 'Enter') enter();
      if (e.key === 'Escape') onclose();
    }}
    type="search"
    placeholder="Type a web address, or words to search the web"
    aria-label="a web address or a search"
    spellcheck="false"
  />
  {#if q.trim()}
    <ul class="rows" aria-label="where to go">
      {#if address}
        <li>
          <button type="button" onclick={() => onweb(`web:${address}`)}
            >Go to {address.replace(/^https:\/\//, '')}</button
          >
        </li>
      {/if}
      <li>
        <button type="button" onclick={() => onweb(`search:${q.trim()}`)}
          >Search the web for “{q.trim()}”</button
        >
      </li>
    </ul>
  {/if}
  <button type="button" class="close" onclick={onclose} aria-label="close">✕</button>
</div>

<style>
  .web-open {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: 12vh var(--space-3) var(--space-3);
    background: var(--bg, Canvas);
    overflow: auto;
  }
  input {
    width: min(40rem, 100%);
    font-size: 1.15rem;
    padding: var(--space-2) var(--space-3);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    width: min(40rem, 100%);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rows button {
    width: 100%;
    text-align: start;
    padding: var(--space-2) var(--space-3);
    min-height: 2.4rem;
    color: var(--accent, LinkText);
  }
  .close {
    position: absolute;
    top: max(var(--safe-top, 0px), var(--space-2));
    right: var(--space-2);
    min-width: 2.6rem;
    min-height: 2.6rem;
  }
</style>

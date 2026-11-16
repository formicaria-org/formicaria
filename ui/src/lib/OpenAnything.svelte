<script lang="ts">
  // **Open…: what a new tab opens** (`decisions.md` 2026-11-16, night, last: *formicaria contains the
  // browser*). ＋ or Ctrl+T in formicaria's browser makes a formicaria tab here: the library and the
  // web in one field. A note found, a section, a new note fill this tab; an address or a web search
  // turns it into a page (`onweb`).
  import { onMount } from 'svelte';
  import { search } from './ipc';
  import type { ObjectMeta } from './types';

  let {
    places,
    onnote,
    onplace,
    onnew,
    onweb,
    onclose,
  }: {
    places: { key: string; label: string; run: () => void }[];
    onnote: (id: string) => void;
    onplace: (run: () => void) => void;
    onnew: () => void;
    /** `web:<url>` to go to, or `search:<words>` for the browser's own search engine. */
    onweb: (at: string) => void;
    onclose: () => void;
  } = $props();

  let q = $state('');
  let found = $state<ObjectMeta[]>([]);
  let field = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  /** What was typed, as a web address, if it reads as one. */
  const address = $derived.by(() => {
    const t = q.trim();
    if (!t || /\s/.test(t)) return null;
    if (/^https?:\/\//i.test(t)) return t;
    return /^[\w-]+(\.[\w-]+)+(:\d+)?(\/.*)?$/.test(t) ? `https://${t}` : null;
  });

  function input() {
    clearTimeout(timer);
    const t = q.trim();
    if (!t) {
      found = [];
      return;
    }
    timer = setTimeout(async () => {
      const mine = ++seq;
      try {
        const got = await search(t);
        if (mine === seq) found = got.slice(0, 8);
      } catch {
        if (mine === seq) found = [];
      }
    }, 200);
  }
  function enter() {
    if (found[0]) return onnote(found[0].id);
    if (address) return onweb(`web:${address}`);
    if (q.trim()) onweb(`search:${q.trim()}`);
  }
  onMount(() => field?.focus());
</script>

<div class="open-anything" role="dialog" aria-label="open">
  <input
    bind:this={field}
    bind:value={q}
    oninput={input}
    onkeydown={(e) => {
      if (e.key === 'Enter') enter();
      if (e.key === 'Escape') onclose();
    }}
    type="search"
    placeholder="Search your notes or the web, or type an address"
    aria-label="search your notes or the web"
    spellcheck="false"
  />
  {#if q.trim()}
    <ul class="rows" aria-label="what to open">
      {#each found as n (n.id)}
        <li>
          <button type="button" onclick={() => onnote(n.id)}>{n.title || 'Untitled'}</button>
        </li>
      {/each}
      {#if address}
        <li>
          <button type="button" class="web" onclick={() => onweb(`web:${address}`)}
            >Go to {address.replace(/^https:\/\//, '')}</button
          >
        </li>
      {/if}
      <li>
        <button type="button" class="web" onclick={() => onweb(`search:${q.trim()}`)}
          >Search the web for “{q.trim()}”</button
        >
      </li>
    </ul>
  {:else}
    <ul class="rows places" aria-label="places">
      <li><button type="button" onclick={onnew}>New note</button></li>
      {#each places as p (p.key)}
        <li><button type="button" onclick={() => onplace(p.run)}>{p.label}</button></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .open-anything {
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
  .places {
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .rows button {
    width: 100%;
    text-align: start;
    padding: var(--space-2) var(--space-3);
    min-height: 2.4rem;
  }
  .places button {
    width: auto;
  }
  .web {
    color: var(--accent, LinkText);
  }
</style>

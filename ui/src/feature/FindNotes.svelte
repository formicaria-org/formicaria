<script lang="ts">
  // **Search, on every page of formicaria's browser** (`ui-audit-2026-11-22.md`, U1). Measured: a
  // computer had no way to search its notes — no field on any page, `/` opened Firefox's find-in-page
  // bar, Ctrl+K opened nothing. The search the older frame has (`App.svelte`, *search is a passing
  // panel*, 2026-10-16) is brought here: a field in the page's bar where there is one (`inline`),
  // and **Ctrl+K / ⌘K or `/`** on every page, a note's included, where it opens over the page.
  // Ctrl+K here finds notes, as quick-find does in other apps; it is not the command palette the
  // app ruled out.
  import { onMount, tick } from 'svelte';
  import { shownVault } from '../lib/shownVaults.svelte';
  import { search } from '../lib/ipc';
  import type { ObjectMeta } from '../lib/types';
  import Search from '../renderers/Search.svelte';
  import Icon from '../lib/Icon.svelte';
  import { webTarget } from '../lib/features';
  import { searchUrl } from '../lib/privacy.svelte';

  // **The one box** (Home, 2026-11-23; `unifying-home-research-2026-11-22.md` §12.3): an address
  // opens the page, and any words also offer *Search the web*. **What Enter does is shown, and
  // chosen** (the owner, the same day: Enter opened a related note when a web search was wanted):
  // the web row is always the first, with its own key (Ctrl+Enter / ⌘↵); the row Enter acts on is
  // drawn as chosen, the top note when notes match; ↑ and ↓ move it.
  let {
    inline = false,
    onopen,
    onweb = (url: string) => void window.open(url, '_blank', 'noopener'),
  }: {
    inline?: boolean;
    onopen: (id: string) => void;
    onweb?: (url: string) => void;
  } = $props();

  let query = $state('');
  let results = $state<ObjectMeta[] | null>(null);
  const web = $derived(webTarget(query, searchUrl));
  /** An address typed (not words): Enter opens it before any note. */
  const address = $derived(!!web && !web.label.startsWith('Search'));
  /** The rows, in order: the web first, then the notes found. */
  const choices = $derived<string[]>([
    ...(web ? ['web'] : []),
    ...(results ?? []).map((n) => n.id),
  ]);
  /** The row Enter acts on: an address, or the top note, or the web when no note matches. */
  let active = $state(0);
  $effect(() => {
    void results;
    void web;
    active = address || !results?.length ? 0 : web ? 1 : 0;
  });
  function act() {
    const c = choices[active];
    if (c === 'web') goWeb();
    else if (c) pick(c);
  }
  function goWeb() {
    if (!web) return;
    const url = web.url;
    close(true);
    onweb(url);
  }
  let open = $state(false); // the results (inline) or the whole box (over the page)
  let field = $state<HTMLInputElement | undefined>(undefined);
  let seq = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const mac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

  function onInput() {
    clearTimeout(timer);
    const q = query.trim();
    if (!q) {
      results = null;
      return;
    }
    open = true;
    timer = setTimeout(async () => {
      const n = ++seq;
      try {
        const found = await search(q);
        if (n === seq) results = found.filter((x) => shownVault(x.vault));
      } catch {
        if (n === seq) results = [];
      }
    }, 200);
  }

  function close(clear = false) {
    open = false;
    if (clear) {
      query = '';
      results = null;
    }
  }

  function pick(id: string) {
    close(true);
    onopen(id);
  }

  async function show() {
    open = true;
    await tick();
    field?.focus();
    field?.select();
  }

  const typing = (t: EventTarget | null) => {
    const el = t as HTMLElement | null;
    return (
      !!el &&
      (el.isContentEditable ||
        /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) ||
        !!el.closest?.('.cm-editor'))
    );
  };

  function onKey(e: KeyboardEvent) {
    const k = e.key.toLowerCase();
    // Taken before the browser's own: Ctrl+K is its web search, `/` its find-in-page bar.
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && k === 'k') {
      e.preventDefault();
      void show();
    } else if (k === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !typing(e.target)) {
      e.preventDefault();
      void show();
    }
  }

  onMount(() => {
    addEventListener('keydown', onKey, true);
    return () => removeEventListener('keydown', onKey, true);
  });
</script>

{#if inline || open}
  <div
    class="find"
    class:over={!inline}
    role={inline ? undefined : 'dialog'}
    aria-label={inline ? undefined : 'search notes'}
  >
    {#if !inline}
      <div class="backdrop" role="presentation" onclick={() => close(true)}></div>
    {/if}
    <div class="box">
      <label class="field">
        <Icon name="search" size={15} />
        <input
          bind:this={field}
          type="search"
          placeholder="Search notes, or type a web address"
          aria-label="search notes"
          spellcheck="false"
          bind:value={query}
          oninput={onInput}
          onfocus={() => query.trim() && (open = true)}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              // Ours alone: a note's page closes the note on Escape (`NotePanel`'s window listener).
              e.preventDefault();
              e.stopPropagation();
              close(!inline || !query.trim());
              field?.blur();
            } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              goWeb();
            } else if (e.key === 'Enter') {
              e.preventDefault();
              act();
            } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
              e.preventDefault();
              const by = e.key === 'ArrowDown' ? 1 : -1;
              active = Math.max(0, Math.min(choices.length - 1, active + by));
            }
          }}
        />
        <kbd title="Search from any page">{mac ? '⌘K' : 'Ctrl K'}</kbd>
      </label>
      {#if open && query.trim()}
        {#if inline}
          <div class="backdrop" role="presentation" onclick={() => close()}></div>
        {/if}
        <div class="panel" role="region" aria-label="search results">
          <div class="head">
            {results === null
              ? 'Searching…'
              : `${results.length} ${results.length === 1 ? 'note' : 'notes'}`}
          </div>
          {#if web}
            <button
              type="button"
              class="web"
              class:active={choices[active] === 'web'}
              aria-current={choices[active] === 'web' ? 'true' : undefined}
              onclick={goWeb}
              ><Icon name="globe" size={14} />
              <span class="label">{web.label}</span>
              <kbd>{mac ? '⌘↵' : 'Ctrl ↵'}</kbd></button
            >
          {/if}
          {#if results !== null}
            <Search
              cards={results}
              {query}
              onopen={pick}
              active={choices[active] !== 'web' ? choices[active] : null}
            />
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .web {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-top: 1px solid var(--border);
    background: none;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
  }
  .web .label {
    flex: 1;
  }
  .web kbd {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .web.active {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .web:hover {
    background: var(--surface);
  }
  .find {
    position: relative;
  }
  .find.over {
    position: fixed;
    inset: 0;
    z-index: 250;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12dvh;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    background: transparent;
  }
  .find.over > .backdrop {
    background: color-mix(in srgb, var(--bg) 55%, transparent);
  }
  .box {
    position: relative;
    z-index: 1;
    width: min(18rem, 40vw);
  }
  .find.over .box {
    width: min(36rem, calc(100vw - 2rem));
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
  }
  .find.over .field {
    padding: var(--space-2) var(--space-3);
    box-shadow: var(--shadow-lg);
  }
  .field input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    font: inherit;
    color: var(--text);
    padding: var(--space-1) 0;
    outline: none;
  }
  kbd {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
    white-space: nowrap;
  }
  .panel {
    position: absolute;
    top: calc(100% + var(--space-1));
    left: 0;
    width: max(100%, 28rem);
    max-width: calc(100vw - 2rem);
    max-height: 60dvh;
    overflow: auto;
    z-index: 2;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .find:not(.over) .panel {
    right: 0;
    left: auto;
  }
  .head {
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-xs);
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }
</style>

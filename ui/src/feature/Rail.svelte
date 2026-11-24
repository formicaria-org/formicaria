<script lang="ts">
  // **The rail** (the shell, P2b; `unifying-home-research-2026-11-22.md` §9.2): where you go, the same
  // on every page. Find or do, Home, Today, the library's sections (how many each holds, and ＋), and
  // Settings. On a narrow screen the same entries sit as a bar along the bottom (`.rail.bar`).
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import sectionsJson from '../lib/sections.json';
  import type { Section, SectionSummary } from '../lib/types';
  import Icon from '../lib/Icon.svelte';
  import { openFind } from '../lib/features';
  import NewMenu from './NewMenu.svelte';
  import SyncStatus from './SyncStatus.svelte';
  import VaultChooser from './VaultChooser.svelte';
  import WaitingDot from '../lib/WaitingDot.svelte';
  import { popup } from '../lib/popup';
  import { hiddenVaults } from '../lib/shownVaults.svelte';
  import { foldRail, railFolded } from '../lib/railFold.svelte';

  let {
    path,
    go,
    onnew,
    onopen,
    unsent = '',
  }: {
    path: string;
    go: (url: string) => void;
    onnew: (s: Section) => void;
    onopen: (id: string) => void;
    /** The backup loop's word on vaults not leaving this device. */
    unsent?: string;
  } = $props();

  const sections = sectionsJson as Section[];
  let summary = $state<SectionSummary[]>([]);
  const read = () =>
    void ipc
      .sectionSummary(hiddenVaults())
      .then((s) => (summary = s))
      .catch(() => {});
  // The counts leave out the vaults chosen not to see, and follow the choice.
  $effect(() => {
    void hiddenVaults().join('\n');
    read();
  });
  onMount(() => {
    const stop = ipc.listenChanges(read);
    // A section's menu closes on a press anywhere else.
    const away = (e: MouseEvent) => {
      const t = e.target as HTMLElement | null;
      if (!t?.closest?.('.more, .menu')) menuFor = null;
    };
    addEventListener('click', away);
    return () => {
      stop();
      removeEventListener('click', away);
    };
  });
  const count = (k: string) => summary.find((s) => s.key === k)?.count;

  // **Sections hidden from the rail** (2026-11-23; the owner: *"I feel like I can only add"*): a
  // section never used leaves the rail, its notes untouched, and comes back from *Show hidden
  // sections*. A view preference of this browser, like the vaults shown.
  const RAIL_KEY = 'fm-rail-hidden';
  let railHidden = $state<string[]>(
    (() => {
      try {
        const v = JSON.parse(localStorage.getItem(RAIL_KEY) ?? '[]');
        return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
      } catch {
        return [];
      }
    })(),
  );
  let showHidden = $state(false);
  let menuFor = $state<string | null>(null);
  /** Where a section's ⋯ is, for its menu to open beside (`popup` keeps it on the screen). */
  function moreBox(key: string) {
    const b = [...document.querySelectorAll<HTMLElement>('button.more')].find(
      (x) => x.dataset.key === key,
    );
    const r = b?.getBoundingClientRect();
    return r
      ? { top: r.top, bottom: r.bottom, left: r.left, right: r.right }
      : { top: 0, bottom: 0, left: 0, right: 0 };
  }
  function setRailHidden(key: string, hide: boolean) {
    railHidden = hide ? [...railHidden, key] : railHidden.filter((k) => k !== key);
    menuFor = null;
    try {
      localStorage.setItem(RAIL_KEY, JSON.stringify(railHidden));
    } catch {
      // Not remembered by this browser; still honoured on this page.
    }
  }
  const listed = $derived(sections.filter((s) => !railHidden.includes(s.key)));
  const tucked = $derived(sections.filter((s) => railHidden.includes(s.key)));
  const on = (url: string) => path === url || path.startsWith(`${url}/`);
  /** Ctrl+K's search, opened from the rail (`FindNotes` listens). */
  const find = openFind;
</script>

<!-- **Folded** (`lib/railFold.svelte.ts`), the rail is a slim strip with one button that brings it
     back, and the page has the width. The panel itself stays drawn at its own width and is clipped
     as the rail narrows, so folding slides rather than reflowing the entries; out of reach (`inert`)
     while folded. -->
<nav class="rail" class:folded={railFolded()} aria-label="formicaria">
  {#if railFolded()}
    <button
      type="button"
      class="unfold"
      aria-label="show the side panel"
      title="Show the side panel (Ctrl+\)"
      onclick={() => foldRail(false)}>☰</button
    >
  {/if}
  <div class="panel" inert={railFolded()}>
    <div class="top">
      <NewMenu {onopen} />
      <button
        type="button"
        class="fold"
        aria-label="hide the side panel"
        title="Hide the side panel (Ctrl+\)"
        onclick={() => foldRail(true)}>«</button
      >
    </div>
    <VaultChooser />
    <button type="button" class="find" onclick={find}
      ><Icon name="search" size={15} /> Find or do <kbd>Ctrl K</kbd></button
    >
    <button
      type="button"
      class="item"
      class:on={on('/f/home')}
      aria-current={on('/f/home') ? 'page' : undefined}
      onclick={() => go('/f/home')}><span class="mark" aria-hidden="true">⌂</span> Home</button
    >
    <button
      type="button"
      class="item"
      class:on={on('/f/today')}
      aria-current={on('/f/today') ? 'page' : undefined}
      onclick={() => go('/f/today')}><span class="mark" aria-hidden="true">◷</span> Journal</button
    >
    <h2>Your library</h2>
    <ul aria-label="sections">
      {#each listed as s (s.key)}
        <li>
          <button
            type="button"
            class="item"
            class:on={on(s.page)}
            aria-current={on(s.page) ? 'page' : undefined}
            onclick={() => go(s.page)}
            ><span class="mark" aria-hidden="true">{s.mark}</span>
            {s.label}{#if s.key === 'collaboration'}<WaitingDot />{/if}
            {#if count(s.key) !== undefined}<span class="n">{count(s.key)}</span>{/if}</button
          >
          <button
            type="button"
            class="add"
            aria-label={`new in ${s.label}`}
            title={`New in ${s.label}`}
            onclick={() => onnew(s)}>＋</button
          >
          <button
            type="button"
            class="more"
            data-key={s.key}
            aria-label={`more for ${s.label}`}
            aria-expanded={menuFor === s.key}
            title={`More for ${s.label}`}
            onclick={() => (menuFor = menuFor === s.key ? null : s.key)}>⋯</button
          >
          {#if menuFor === s.key}
            <div
              class="menu"
              role="menu"
              aria-label={`${s.label} in the rail`}
              use:popup={{ anchor: () => moreBox(s.key), align: 'end' }}
            >
              <button type="button" role="menuitem" onclick={() => setRailHidden(s.key, true)}
                >Hide from the rail</button
              >
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if tucked.length}
      <button
        type="button"
        class="tucked-toggle"
        aria-expanded={showHidden}
        onclick={() => (showHidden = !showHidden)}
        >{showHidden ? 'Hide' : 'Show'} hidden sections ({tucked.length})</button
      >
      {#if showHidden}
        <ul class="tucked" aria-label="hidden sections">
          {#each tucked as s (s.key)}
            <li>
              <button type="button" class="item" onclick={() => go(s.page)}
                ><span class="mark" aria-hidden="true">{s.mark}</span> {s.label}</button
              >
              <button
                type="button"
                class="unhide"
                aria-label={`show ${s.label} in the rail`}
                onclick={() => setRailHidden(s.key, false)}>Show</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
    <span class="grow"></span>
    <SyncStatus {unsent} />
    <!-- In formicaria's browser Settings is the gear at the top right of every tab (the owner,
       2026-11-18), and a second gear here was one door too many (2026-11-24), so it steps aside
       there: the bridge marks the page `data-formicaria-bridge` (style below). In any other
       browser it is the only way in, so it stays. -->
    <button
      type="button"
      class="item settings"
      class:on={on('/f/settings')}
      aria-current={on('/f/settings') ? 'page' : undefined}
      onclick={() => go('/f/settings')}
      ><span class="mark" aria-hidden="true">⚙</span> Settings</button
    >
  </div>
</nav>

<style>
  /* The rail is the frame and the clip; `.panel` is what it holds, always its full width. Folding
     narrows the rail and the panel slides out under its edge (none of it when motion is reduced). */
  .rail {
    position: relative;
    width: 14.5rem;
    flex: none;
    box-sizing: border-box;
    border-right: 1px solid var(--border);
    background: var(--card-bg);
    overflow: hidden;
    height: 100%;
  }
  .rail.folded {
    width: 2.75rem;
  }
  @media (prefers-reduced-motion: no-preference) {
    .rail {
      transition: width 0.18s ease;
    }
    /* Seen while it slides out, out of sight once it has. */
    .rail.folded .panel {
      transition: visibility 0s linear 0.18s;
    }
  }
  .rail.folded .panel {
    visibility: hidden;
  }
  .unfold {
    position: absolute;
    top: var(--space-2);
    left: 50%;
    translate: -50% 0;
    z-index: 1;
    width: 2rem;
    height: 2rem;
    font: inherit;
    font-size: 1.05rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .top {
    display: flex;
    gap: var(--space-1);
    align-items: stretch;
  }
  .top > :global(.new) {
    flex: 1;
    min-width: 0;
  }
  .fold {
    flex: none;
    width: 2rem;
    font: inherit;
    font-size: 1.05rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
    cursor: pointer;
  }
  .fold:hover,
  .unfold:hover {
    color: var(--text);
    border-color: var(--text-muted);
  }
  :global(html[data-formicaria-bridge]) .settings {
    display: none;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 14.5rem;
    padding: var(--space-2);
    box-sizing: border-box;
    overflow: auto;
    height: 100%;
  }
  .find {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-1) 0 var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
    font: inherit;
    cursor: pointer;
  }
  kbd {
    margin-left: auto;
    font: var(--text-xs) var(--font-sans);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
  }
  h2 {
    margin: var(--space-3) var(--space-2) var(--space-1);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    position: relative;
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: none;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--surface);
  }
  .item.on {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
    font-weight: 600;
  }
  .mark {
    width: 1.2rem;
    text-align: center;
  }
  .n {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  .add {
    position: absolute;
    right: var(--space-1);
    top: 50%;
    translate: 0 -50%;
    display: none;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
  }
  li:hover .add,
  .add:focus-visible,
  li:hover .more,
  .more:focus-visible,
  .more[aria-expanded='true'] {
    display: block;
  }
  .more {
    position: absolute;
    right: 1.9rem;
    top: 50%;
    translate: 0 -50%;
    display: none;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
  }
  .menu {
    position: absolute;
    z-index: 30;
    right: 0;
    top: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md, 0 4px 16px rgb(0 0 0 / 0.15));
    padding: var(--space-1);
  }
  .menu button {
    font: inherit;
    background: none;
    border: none;
    color: var(--text);
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    white-space: nowrap;
  }
  .menu button:hover {
    background: var(--card-bg);
  }
  .tucked-toggle {
    font: inherit;
    font-size: var(--text-xs);
    color: var(--text-muted);
    background: none;
    border: none;
    text-align: left;
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
  }
  .tucked {
    list-style: none;
    margin: 0;
    padding: 0;
    opacity: 0.8;
  }
  .tucked li {
    display: flex;
    align-items: center;
  }
  .unhide {
    font: inherit;
    font-size: var(--text-xs);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  li:hover .n {
    visibility: hidden;
  }
  .grow {
    flex: 1;
  }
  /* A narrow screen: the rail is hidden; the page's own bar keeps the chooser (P2c brings the
     bottom bar). */
  @media (max-width: 760px) {
    .rail {
      display: none;
    }
  }
</style>

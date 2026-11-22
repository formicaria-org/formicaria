<script lang="ts">
  // **The rail** (the shell, P2b; `unifying-home-research-2026-11-22.md` §9.2): where you go, the same
  // on every page. Find or do, Home, Today, the library's sections (how many each holds, and ＋), and
  // Settings. On a narrow screen the same entries sit as a bar along the bottom (`.rail.bar`).
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import sectionsJson from '../lib/sections.json';
  import type { Section, SectionSummary } from '../lib/types';
  import Icon from '../lib/Icon.svelte';
  import NewMenu from './NewMenu.svelte';
  import SyncStatus from './SyncStatus.svelte';

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
      .sectionSummary()
      .then((s) => (summary = s))
      .catch(() => {});
  onMount(() => {
    read();
    return ipc.listenChanges(read);
  });
  const count = (k: string) => summary.find((s) => s.key === k)?.count;
  const on = (url: string) => path === url || path.startsWith(`${url}/`);
  /** Ctrl+K's search, opened from the rail (`FindNotes` listens). */
  const find = () =>
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
</script>

<nav class="rail" aria-label="formicaria">
  <div class="space" title="Spaces come later: one space, everything in it">Everything</div>
  <NewMenu {onopen} />
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
    onclick={() => go('/f/today')}><span class="mark" aria-hidden="true">◷</span> Today</button
  >
  <h2>Your library</h2>
  <ul aria-label="sections">
    {#each sections as s (s.key)}
      <li>
        <button
          type="button"
          class="item"
          class:on={on(s.page)}
          aria-current={on(s.page) ? 'page' : undefined}
          onclick={() => go(s.page)}
          ><span class="mark" aria-hidden="true">{s.mark}</span>
          {s.label}
          {#if count(s.key) !== undefined}<span class="n">{count(s.key)}</span>{/if}</button
        >
        <button
          type="button"
          class="add"
          aria-label={`new in ${s.label}`}
          title={`New in ${s.label}`}
          onclick={() => onnew(s)}>＋</button
        >
      </li>
    {/each}
  </ul>
  <span class="grow"></span>
  <SyncStatus {unsent} />
  <button
    type="button"
    class="item"
    class:on={on('/f/settings')}
    aria-current={on('/f/settings') ? 'page' : undefined}
    onclick={() => go('/f/settings')}
    ><span class="mark" aria-hidden="true">⚙</span> Settings</button
  >
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 14.5rem;
    flex: none;
    padding: var(--space-2);
    box-sizing: border-box;
    border-right: 1px solid var(--border);
    background: var(--card-bg);
    overflow: auto;
    height: 100%;
  }
  .space {
    font-weight: 650;
    padding: var(--space-1) var(--space-2);
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
  .add:focus-visible {
    display: block;
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

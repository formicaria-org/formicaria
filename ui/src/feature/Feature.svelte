<script lang="ts">
  import { popup } from '../lib/popup';
  // **One feature, or one note, filling a tab** (`decisions.md` 2026-11-17, *one server, many
  // views*). The address says which (`route.ts`); the page holds no state beyond what is on screen:
  // it reads by commands, writes by commands, and re-reads when the server says something moved
  // (`listenChanges`). Opening a note opens it in a tab of its own (`openItem`).
  import { onMount } from 'svelte';
  import Pane from '../lib/Pane.svelte';
  import { feedKey, newPane, type Feed, type Pane as PaneT } from '../lib/panes';
  import { loadFeed, groupsOf, statusesOf } from '../lib/feeds';
  import { BUILTIN_SPACES, sectionLabel } from '../lib/spaces.svelte';
  import * as ipc from '../lib/ipc';
  import type { ViewInfo } from '../lib/types';
  import { parseRoute, itemUrl, type Route } from './route';
  import { setPropertyUndoable } from '../lib/undo.svelte';
  import { trackersShown } from '../lib/trackers.svelte';
  import { FEATURES, featureOf } from '../lib/features';
  import NewTab from './NewTab.svelte';
  import SelectionBring from '../lib/SelectionBring.svelte';
  import { noteShown } from '../lib/lastNote';

  let { go = (url: string) => location.assign(url) }: { go?: (url: string) => void } = $props();

  const route: Route = parseRoute(location.pathname);

  let views = $state<ViewInfo[]>([]);
  let vaults = $state<string[]>([]);
  let pane = $state<PaneT>(
    route.kind === 'note'
      ? newPane('note', { noteId: route.id })
      : route.kind === 'view'
        ? newPane('view', { viewName: route.name })
        : route.kind === 'builtin'
          ? newPane(route.pane)
          : newPane('timeline'),
  );
  let feed = $state<Feed | undefined>(undefined);
  let error = $state('');

  /** The feature's own views, as a strip: the group's (shipped or saved) and built-in ones. */
  const tabs = $derived.by(() => {
    if (route.kind !== 'group') return [];
    const builtin = BUILTIN_SPACES.find((s) => s.name === route.name);
    const shipped = groupsOf(views, trackersShown()).find((g) => g.name === route.name);
    return [
      ...(shipped?.views ?? []).map((v) => ({
        label: v,
        to: { kind: 'view' as const, viewName: v },
      })),
      ...(builtin?.tabs ?? [])
        .filter((t) => !t.viewName)
        .map((t) => ({ label: t.label, to: { kind: t.kind, viewName: null } })),
    ];
  });

  // A group opens on its first view **once**, when the list of views has come. (Asked on every
  // change, a first view that is the timeline set the page to the timeline again and again.)
  let opened = false;
  $effect(() => {
    if (opened || route.kind !== 'group' || !tabs.length) return;
    opened = true;
    const first = tabs[0].to;
    pane = newPane(first.kind, first.viewName ? { viewName: first.viewName } : {});
  });

  /** The feature this tab shows, for its chooser (`Food ▾`). */
  const here = featureOf(location.pathname);
  let choosing = $state(false);
  // Settings as a page: its theme and the new-vault form, as the app keeps them.
  let theme = $state(
    typeof document !== 'undefined' ? (document.documentElement.dataset.theme ?? 'dark') : 'dark',
  );
  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('fm-theme', theme);
    } catch {
      /* this device keeps no choice */
    }
  }
  let newVaultOpen = $state(false);

  async function read() {
    // A feature reads once its first view is chosen, not the placeholder before it.
    if (route.kind === 'group' && !opened) return;
    const key = feedKey(pane);
    if (!key) {
      feed = undefined;
      return;
    }
    try {
      feed = await loadFeed(key);
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    void feedKey(pane);
    void read();
  });

  // **The tab's title is the thing it shows**, as a browser tab names its page.
  $effect(() => {
    const name =
      pane.kind === 'note'
        ? pane.noteTitle || 'Note'
        : route.kind === 'group'
          ? sectionLabel(route.name)
          : (pane.viewName ?? pane.kind);
    document.title = `${name} — formicaria`;
  });

  /** Open a note (or anything) in a tab of its own. */
  function openNote(id: string) {
    window.open(itemUrl({ kind: 'note', id }), '_blank', 'noopener');
  }

  let commitTimer: ReturnType<typeof setTimeout> | undefined;
  /** A write is recorded in history a few seconds after the last one, as in the app. */
  function saved() {
    clearTimeout(commitTimer);
    commitTimer = setTimeout(() => {
      for (const v of vaults)
        void ipc.commit(`auto: ${new Date().toISOString()}`, v).catch(() => {});
    }, 5000);
  }

  async function move(groupBy: string, id: string, value: string) {
    try {
      await setPropertyUndoable(id, groupBy, value, `moved a card to “${value || 'none'}”`);
      saved();
      await read();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void ipc
      .listViews()
      .then((v) => (views = v))
      .catch(() => {});
    void ipc
      .listVaults()
      .then((v) => (vaults = v.map((x) => x.name)))
      .catch(() => {});
    const stop = ipc.listenChanges(() => void read());
    // Leaving the page: a write still owed to history is recorded now.
    const flush = () => {
      if (commitTimer) {
        clearTimeout(commitTimer);
        for (const v of vaults)
          void ipc.commit(`auto: ${new Date().toISOString()}`, v).catch(() => {});
      }
    };
    addEventListener('pagehide', flush);
    return () => {
      stop();
      removeEventListener('pagehide', flush);
    };
  });

  const active = (t: { to: { kind: string; viewName: string | null } }) =>
    t.to.viewName
      ? pane.kind === 'view' && pane.viewName === t.to.viewName
      : pane.kind === t.to.kind;
</script>

<div class="feature">
  {#if route.kind === 'new'}
    <NewTab {go} />
  {:else if route.kind === 'settings'}
    {#await import('../lib/SettingsPanel.svelte') then { default: SettingsPanel }}
      <SettingsPanel
        layout="single"
        onlayout={() => {}}
        onclose={() => (history.length > 1 ? history.back() : window.close())}
        onkeyschanged={() => {}}
        commands={[]}
        columns="auto"
        oncolumns={() => {}}
        {theme}
        ontheme={toggleTheme}
        onnewvault={() => (newVaultOpen = true)}
      />
    {/await}
    {#if newVaultOpen}
      <div class="sheet" role="dialog" aria-modal="true" aria-label="New vault">
        {#await import('../lib/NewVault.svelte') then { default: NewVault }}
          <NewVault
            oncreated={() => (newVaultOpen = false)}
            oncancel={() => (newVaultOpen = false)}
          />
        {/await}
      </div>
    {/if}
  {:else}
    {#if route.kind !== 'note'}
      <!-- **The tab's line** (`decisions.md` 2026-11-17, *one frame*): which feature this tab shows, as
         a chooser that moves *this* tab (Back returns), then the feature's own views. -->
      <nav
        class="strip"
        aria-label="{route.kind === 'group' ? sectionLabel(route.name) : ''} views"
      >
        <div class="chooser">
          <button
            type="button"
            class="feature-name"
            aria-haspopup="menu"
            aria-expanded={choosing}
            aria-label="switch this tab to another feature"
            onclick={() => (choosing = !choosing)}
            >{here?.label ??
              (route.kind === 'group' ? sectionLabel(route.name) : (pane.viewName ?? pane.kind))} ▾</button
          >
          {#if choosing}
            <ul class="menu" role="menu" aria-label="features" use:popup>
              {#each FEATURES as f (f.key)}
                <li role="none">
                  <button type="button" role="menuitem" onclick={() => go(f.url)}>{f.label}</button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
        <!-- A tracker (Food, Fitness, Health) shows its own views inside it: not repeated here. -->
        {#if tabs.length > 1 && feed?.view?.renderer !== 'totals'}
          {#each tabs as t (t.label)}
            <button
              type="button"
              class:on={active(t)}
              aria-current={active(t) ? 'page' : undefined}
              onclick={() =>
                (pane = newPane(
                  t.to.kind as PaneT['kind'],
                  t.to.viewName ? { viewName: t.to.viewName } : {},
                ))}>{t.label}</button
            >
          {/each}
        {/if}
      </nav>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="body">
      <Pane
        {pane}
        index={0}
        cols={1}
        {feed}
        statuses={statusesOf(feed)}
        shown={() => true}
        focused={true}
        startEditing={false}
        {vaults}
        onopen={openNote}
        onresolve={async () => {}}
        onmove={(g, id, v) => void move(g, id, v)}
        onstatus={(id, v) => void move('status', id, v ?? '')}
        onnavigate={openNote}
        onsaved={saved}
        onchange={(patch) => {
          // Only a real change: a new object for the same pane would draw it again, and again.
          const changed = Object.entries(patch).some(([k, v]) => pane[k as keyof PaneT] !== v);
          if (changed) pane = { ...pane, ...patch };
        }}
        ontitle={(t) => {
          if (t !== pane.noteTitle) pane = { ...pane, noteTitle: t };
          if (route.kind === 'note') noteShown(route.id, t || 'Untitled');
        }}
        onreorder={() => {}}
        onresize={() => {}}
        onclose={() => window.close()}
        onfocus={() => {}}
        headed={false}
      />
    </div>
  {/if}
</div>

<SelectionBring
  source={() =>
    route.kind === 'note' ? { id: route.id, title: pane.noteTitle || 'a note' } : null}
/>

<style>
  .feature {
    display: flex;
    flex-direction: column;
    height: 100dvh;
    min-height: 0;
  }
  .strip {
    display: flex;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    overflow-x: auto;
    flex: none;
  }
  .strip button {
    flex: none;
    padding: var(--space-1) var(--space-3);
    border-radius: 999px;
    min-height: 2rem;
  }
  .chooser {
    position: relative;
    flex: none;
  }
  .feature-name {
    font-weight: 700;
  }
  .menu {
    list-style: none;
    margin: 4px 0 0;
    padding: 4px;
    min-width: 12rem;
    background: var(--bg, Canvas);
    border-radius: 10px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.25);
  }
  .menu button {
    width: 100%;
    text-align: start;
    border-radius: 6px;
  }
  .sheet {
    position: fixed;
    inset: 5vh 5vw;
    z-index: 30;
    overflow: auto;
    background: var(--bg, Canvas);
    border-radius: 12px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.3);
    padding: var(--space-3);
  }
  .strip button.on {
    background: var(--accent-soft, color-mix(in srgb, #dc2626 18%, transparent));
    font-weight: 600;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .body > :global(.pane) {
    flex: 1;
    min-width: 0;
  }
  .error {
    margin: var(--space-2);
  }
</style>

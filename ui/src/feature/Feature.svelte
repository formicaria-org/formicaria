<script lang="ts">
  import { toggledTheme } from '../lib/start';
  import UpdateBanner from '../lib/UpdateBanner.svelte';
  import BrowserRestartBanner from '../lib/BrowserRestartBanner.svelte';
  import { watchForUpdates, update as startUpdate } from '../lib/updater.svelte';
  import { resumeQueue as resumeMediaQueue } from '../lib/mediaRun';
  import { untrack as untrackUpdates } from 'svelte';
  // **A newer version is offered on every page** (the owner on 0.49/0.50: *"impossible for me to
  // upgrade from within, all options disappeared"*). The check and the banner lived in the old
  // whole-app page, which formicaria's browser no longer shows; each feature page watches now.
  $effect(() => untrackUpdates(watchForUpdates));
  // **Save media's queue, carried on** (2026-11-21): tracks a closed tab had not finished; one tab
  // only (`mediaRun.resumeQueue` claims the queue).
  $effect(() => {
    const carryOn = () => void resumeMediaQueue();
    carryOn();
    const onVisible = () => document.visibilityState === 'visible' && carryOn();
    document.addEventListener('visibilitychange', onVisible);
    const timer = setInterval(carryOn, 10000);
    return () => {
      document.removeEventListener('visibilitychange', onVisible);
      clearInterval(timer);
    };
  });
  // **`#update`**: the browser's own bar was pressed (`formicaria.cfg`, `offerUpdate`, 2026-11-21) —
  // the one press, done here where open notes are saved first; on load, or on a tab already open.
  $effect(() => {
    const take = () => {
      if (location.hash !== '#update') return;
      history.replaceState(history.state, '', location.pathname + location.search);
      void untrackUpdates(startUpdate);
    };
    take();
    window.addEventListener('hashchange', take);
    return () => window.removeEventListener('hashchange', take);
  });
  import SaveMedia from '../lib/SaveMedia.svelte';
  import type { Candidate } from '../lib/web/media';
  /** What the add-on found on the page (`#` + JSON: the page and its media). */
  function mediaAsked(): { page: string; c: Candidate[] } | null {
    try {
      const v = JSON.parse(decodeURIComponent(location.hash.slice(1))) as {
        page?: unknown;
        c?: unknown;
      };
      return typeof v.page === 'string' && Array.isArray(v.c)
        ? { page: v.page, c: v.c as Candidate[] }
        : null;
    } catch {
      return null;
    }
  }
  import PrivacySheet from '../lib/PrivacySheet.svelte';
  import { popup } from '../lib/popup';
  // **One feature, or one note, filling a tab** (`decisions.md` 2026-11-17, *one server, many
  // views*). The address says which (`route.ts`); the page holds no state beyond what is on screen:
  // it reads by commands, writes by commands, and re-reads when the server says something moved
  // (`listenChanges`). Opening a note opens it in a tab of its own (`openItem`).
  import { onMount, untrack } from 'svelte';
  import * as diary from '../lib/diary';
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
  import SectionNotes from './SectionNotes.svelte';
  import ViewControls from '../lib/ViewControls.svelte';
  import HomeView from './HomeView.svelte';
  import TodayView from './TodayView.svelte';
  import sectionsJson from '../lib/sections.json';
  import type { Section, SectionView } from '../lib/types';
  import SelectionBring from '../lib/SelectionBring.svelte';
  import { noteShown, lastNote } from '../lib/lastNote';
  import FindNotes from './FindNotes.svelte';
  import { refreshVaults } from '../lib/vaults.svelte';
  import UndoDelete from './UndoDelete.svelte';
  import { backUpInOneTab } from './backupLoop';
  import { sync as syncReminders } from '../lib/reminders.svelte';
  import ReminderBanner from '../lib/ReminderBanner.svelte';
  import ManualPanel from '../lib/ManualPanel.svelte';
  import { unsentLabel, type Unsent } from '../lib/autoBackup';

  let {
    go = (url: string) => location.assign(url),
    path = location.pathname,
    onitem,
  }: {
    go?: (url: string) => void;
    /** The address this page draws (the shell passes it; on its own, the page's own). */
    path?: string;
    /** Open a note beside the page (the shell); without it, in a tab of its own. */
    onitem?: (id: string) => void;
  } = $props();
  /// **Inside the shell**, the work that must outlive a move between places (the backup loop, the
  /// reminders, the manual, the undo toast) is the shell's; on its own, the page does it.

  /** **Leaving a feature page never closes the window** (2026-11-19). It went back, or else closed
   *  the window: a page opened on its own (restored at start, one entry in its history) closed the
   *  only window, and the browser with it, and the next start restored the same page. Back if there
   *  is somewhere to go back to, else the notes. */
  const leave = () => (history.length > 1 ? history.back() : go('/f/home'));

  // **Settings is never nowhere** (2026-11-19; the owner, after 0.55.0: *"One should update and have
  // everything there automatically"*). It is the window's, at the right end of the tab row; while that
  // button cannot be there (formicaria's browser not the one open, or still on an older kit) the page
  // shows its own. Unknown, or the question failing, counts as not there.
  let windowHasSettings = $state(true);
  onMount(() => {
    // **The usage diary** (`lib/diary.ts`): this page's kind and how long it took to open — a
    // feature's own name, never a note's, a group's or a view's (those are the person's).
    diary.watch();
    diary.pageOpened(featureOf(path)?.key ?? (route.kind === 'builtin' ? route.pane : route.kind));
    ipc.browserStatus().then(
      (st) => (windowHasSettings = !!(st.running && st.kit_current)),
      () => (windowHasSettings = false),
    );
  });

  // Drawn again, not changed, when the address moves (the shell keys the page on it).
  const route: Route = parseRoute(untrack(() => path));

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
  const here = featureOf(untrack(() => path));
  /// **Its section** (the shell, P2a): the same feature as a section of the library, with its generic
  /// views (a table, a gallery) beside its own, and what its ＋ makes.
  const section = (sectionsJson as Section[]).find((s) => s.page === here?.url);
  const genericViews = (section?.views ?? []).filter((v) => !!v.renderer);
  /// The generic view chosen, or none for the section's own ("As usual").
  let generic = $state<SectionView | null>(null);
  const viewName = (v: SectionView) =>
    v.label ?? (v.renderer ? v.renderer.charAt(0).toUpperCase() + v.renderer.slice(1) : '');
  let choosing = $state(false);
  // Settings as a page: its theme and the new-vault form, as the app keeps them.
  let theme = $state(
    typeof document !== 'undefined' ? (document.documentElement.dataset.theme ?? 'dark') : 'dark',
  );
  function toggleTheme() {
    theme = toggledTheme(theme);
  }
  let newVaultOpen = $state(false);
  let newVaultStart = $state<'create' | 'import'>('create');

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

  // **The tab's title is the thing it shows**, as a browser tab names its page. The pages that are
  // not a view say what they are: they read "timeline" before (the placeholder pane's kind; the
  // audit, 2026-11-22).
  const PAGE_NAMES: Partial<Record<Route['kind'], string>> = {
    new: 'New tab',
    home: 'Home',
    today: 'Today',
    settings: 'Settings',
    privacy: 'Privacy',
    media: 'Save media',
  };
  $effect(() => {
    const name =
      PAGE_NAMES[route.kind] ??
      (pane.kind === 'note'
        ? pane.noteTitle || 'Note'
        : route.kind === 'group'
          ? sectionLabel(route.name)
          : (here?.label ?? pane.viewName ?? pane.kind));
    document.title = `${name} — formicaria`;
  });
  /** The pages with a bar of their own carry the search field in it; the others open it over the
   *  page with Ctrl+K or `/` (`FindNotes.svelte`). */
  const hasStrip = !['new', 'media', 'privacy', 'settings', 'note', 'home', 'today'].includes(
    route.kind,
  );
  /** *Recently deleted*, from the feature chooser (`DeletedPanel`, `decisions.md` 2026-10-09). */
  let deletedOpen = $state(false);

  /// **Backing up by itself, on the computer** (`backupLoop.ts`): one tab of formicaria's browser runs
  /// the loop; a vault whose changes are not leaving is said here, as the older frame's chip says it.
  let unsent = $state<Unsent[]>([]);
  let backupNote = $state('');
  onMount(() => {
    if (onitem) return;
    return backUpInOneTab({
      refresh: () => void read(),
      onUnsent: (u) => (unsent = u),
      // **The program's reminders kept current** (found 2026-11-22): on a computer the program says a
      // reminder when no page is open, from the list a page hands it. Only the older frame handed
      // it, so on formicaria's browser that list went stale. The tab holding the loop hands it, at
      // start and whenever the notes change.
      whileLeading: () => {
        void syncReminders();
        return ipc.listenChanges(() => void syncReminders());
      },
      said: (vault, phase) => {
        if (phase === 'conflicts')
          backupNote = `Notes in “${vault}” came back with edits from another device in both versions: open them to merge.`;
      },
    });
  });

  /** Open a note (or anything) in a tab of its own. */
  function openNote(id: string) {
    if (onitem) onitem(id);
    else window.open(itemUrl({ kind: 'note', id }), '_blank', 'noopener');
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
    // The vault list, for the vault badge (drawn only with more than one vault).
    void refreshVaults();
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

  /** **＋ New note** (the view's line): a blank note, opened in a tab of its own. */
  async function newNote() {
    try {
      // **What this section's ＋ makes** (its `.kind`'s `new:`): a task on Tasks, an entry on Food.
      const props = (section?.new?.props ?? {}) as Record<string, string | number>;
      const [made] = await ipc.createNotes([{ title: '', body: '', tags: [], start: '', props }]);
      if (made) window.open(`/f/note/${made.id}`, '_blank');
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="feature">
  {#if !onitem}<UndoDelete onrestored={() => void read()} />{/if}
  {#if !hasStrip}
    <FindNotes onopen={openNote} />
  {/if}
  {#if deletedOpen}
    <div class="sheet" role="dialog" aria-modal="true" aria-label="Recently deleted">
      {#await import('../lib/DeletedPanel.svelte') then { default: DeletedPanel }}
        <DeletedPanel onclose={() => (deletedOpen = false)} onrestored={() => void read()} />
      {/await}
    </div>
  {/if}
  <UpdateBanner />
  <BrowserRestartBanner />
  {#if !onitem}<ReminderBanner />{/if}
  <!-- The manual's reader: every "How this works" link opens it (dead on these pages before
       2026-11-22: the panel was only mounted by the older frame). -->
  {#if !onitem}<ManualPanel />{/if}
  {#if unsent.length}
    <p class="backup-warn" role="status">
      {unsentLabel(unsent)} · <a href="/f/settings">Back up in Settings</a>
    </p>
  {/if}
  {#if backupNote}
    <p class="backup-warn" role="status">
      {backupNote}
      <button type="button" aria-label="dismiss" onclick={() => (backupNote = '')}>✕</button>
    </p>
  {/if}
  {#if route.kind === 'new'}
    <NewTab {go} />
  {:else if route.kind === 'home'}
    <HomeView {go} onopen={openNote} />
  {:else if route.kind === 'today'}
    <TodayView onopen={openNote} />
  {:else if route.kind === 'media'}
    {@const asked = mediaAsked()}
    <div class="privacy-page">
      <h1>Save media</h1>
      {#if asked}
        <SaveMedia
          page={asked.page}
          candidates={asked.c}
          note={lastNote()}
          onopen={(id) => go(`/f/note/${id}`)}
        />
      {:else}
        <p class="muted">Open a web page, then the ant's <strong>Save media</strong>.</p>
      {/if}
    </div>
  {:else if route.kind === 'privacy'}
    <div class="privacy-page">
      <h1>Privacy</h1>
      <PrivacySheet computer />
      <p class="muted">
        Firefox's shield in the address bar shows what was blocked on the page you are on.
      </p>
    </div>
  {:else if route.kind === 'settings'}
    {#await import('../lib/SettingsPanel.svelte') then { default: SettingsPanel }}
      <SettingsPanel
        layout="single"
        onlayout={() => {}}
        onclose={leave}
        page
        place={!!onitem}
        onkeyschanged={() => {}}
        commands={[]}
        columns="auto"
        oncolumns={() => {}}
        {theme}
        ontheme={toggleTheme}
        onnewvault={(start) => {
          newVaultStart = start ?? 'create';
          newVaultOpen = true;
        }}
      />
    {/await}
    {#if newVaultOpen}
      <div class="sheet" role="dialog" aria-modal="true" aria-label="New vault">
        {#await import('../lib/NewVault.svelte') then { default: NewVault }}
          <NewVault
            start={newVaultStart}
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
              <li role="separator" class="sep"></li>
              <li role="none">
                <button
                  type="button"
                  role="menuitem"
                  onclick={() => ((choosing = false), (deletedOpen = true))}
                  >Recently deleted…</button
                >
              </li>
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
              onclick={() => {
                generic = null;
                pane = newPane(
                  t.to.kind as PaneT['kind'],
                  t.to.viewName ? { viewName: t.to.viewName } : {},
                );
              }}>{t.label}</button
            >
          {/each}
        {/if}
        {#if !generic}
          <!-- **The view's own controls** (the parity list: Agenda's month, week or list, the
               timeline's feed or list, a saved view's "show everything"), as the older frame's bar. -->
          <span class="controls">
            <ViewControls
              {pane}
              {feed}
              onchange={(patch) => {
                const changed = Object.entries(patch).some(
                  ([k, v]) => pane[k as keyof PaneT] !== v,
                );
                if (changed) pane = { ...pane, ...patch };
              }}
            />
          </span>
        {/if}
        {#if genericViews.length}
          <!-- **View as** (the shell, P2a): every section's notes as a table or a gallery, beside its own. -->
          <span class="view-as" role="group" aria-label="view as">
            <button
              type="button"
              class:on={!generic}
              aria-pressed={!generic}
              onclick={() => (generic = null)}>As usual</button
            >
            {#each genericViews as v (viewName(v))}
              <button
                type="button"
                class:on={generic === v}
                aria-pressed={generic === v}
                onclick={() => (generic = v)}>{viewName(v)}</button
              >
            {/each}
          </span>
        {/if}
        <!-- **Help from every view**, a plain link. Settings is the window's, at the right end of the
             tab row on every tab (owner, 2026-11-18: *"why not simply add settings on top right as
             per usual apps"*; `browser/kit/formicaria.cfg`), and here only while the window cannot
             show it (`windowHasSettings`). -->
        <span class="system">
          <FindNotes inline onopen={openNote} />
          <!-- **A new note from every view** (owner on 0.52: *"how are notes added? The plus
               disappeared from every view"*): in a tab of its own, straight in the editor. -->
          <button type="button" class="new-note" onclick={() => void newNote()} title="A new note"
            >＋ New note</button
          >
          <a href="/manual/" target="_blank" rel="noopener" title="Help: how formicaria works"
            >Help</a
          >
          {#if !windowHasSettings}
            <a href="/f/settings" title="Settings: privacy, updates, mail for meetings, and more"
              >⚙ Settings</a
            >
          {/if}
        </span>
      </nav>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="body">
      {#if generic && section}
        <SectionNotes {section} view={generic} onopen={openNote} />
      {:else}
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
          onresolve={async (c, keep) => {
            // **A conflict's buttons do what they say** (they were a no-op on these pages until
            // 2026-11-22): the same `resolve_conflict` the older frame calls, then a fresh read.
            try {
              await ipc.resolveConflict(c.vault, c.path, keep);
              await read();
              saved();
            } catch (e) {
              error = String(e);
            }
          }}
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
          onclose={leave}
          onfocus={() => {}}
          headed={false}
        />
      {/if}
    </div>
  {/if}
</div>

<SelectionBring
  source={() =>
    route.kind === 'note' ? { id: route.id, title: pane.noteTitle || 'a note' } : null}
/>

<style>
  .privacy-page {
    max-width: 44rem;
    margin: 0 auto;
    padding: 1.5rem 1rem;
  }
  .privacy-page .muted {
    opacity: 0.7;
    margin-top: 1rem;
  }
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
  .system {
    margin-inline-start: auto;
    display: flex;
    gap: var(--space-2);
    align-items: center;
    flex: none;
    padding-inline-start: var(--space-2);
  }
  .system a {
    padding: var(--space-1) var(--space-2);
    border-radius: 999px;
    color: inherit;
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
  .menu .sep {
    border-top: 1px solid var(--border);
    margin: var(--space-1) 0;
  }
  .view-as {
    display: inline-flex;
    gap: 2px;
    margin-inline-start: var(--space-2);
    padding: 2px;
    border-radius: var(--radius-md);
    background: var(--card-bg);
  }
  .view-as button {
    border: none;
    background: transparent;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .view-as button.on {
    background: var(--surface);
    font-weight: 600;
  }
  .backup-warn {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    background: var(--warning-bg, #fff4d6);
    color: var(--text);
    font-size: var(--text-sm);
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }
  .backup-warn button {
    margin-left: auto;
    border: none;
    background: transparent;
    cursor: pointer;
  }
</style>

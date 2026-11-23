<script lang="ts">
  // **The shell** (P2b; `decisions.md` 2026-11-22, evening; `unifying-home-research-2026-11-22.md`
  // §9.2, §10). One interface on every page of formicaria's browser: the rail (where you go), the
  // page it chose (a section's views, Home, Today, Settings…), and the item beside it on a wide
  // screen. Moving between places stays in the tab: the address changes (Back works) and the page is
  // drawn again, with no reload. The page itself is `Feature.svelte`, unchanged in what it does.
  import { onMount } from 'svelte';
  import Feature from './Feature.svelte';
  import Rail from './Rail.svelte';
  import PlayerBar from '../lib/music/PlayerBar.svelte';
  import * as undo from '../lib/undo.svelte';
  import UndoDelete from './UndoDelete.svelte';
  import ReminderBanner from '../lib/ReminderBanner.svelte';
  import ManualPanel from '../lib/ManualPanel.svelte';
  import { backUpInOneTab } from './backupLoop';
  import { sync as syncReminders } from '../lib/reminders.svelte';
  import { unsentLabel } from '../lib/autoBackup';
  import ShareSheet from '../lib/ShareSheet.svelte';
  import { piecesMarkdown, type Piece } from '../lib/piece';
  import Pane from '../lib/Pane.svelte';
  import { newPane, type Pane as PaneT } from '../lib/panes';
  import * as ipc from '../lib/ipc';
  import type { Section } from '../lib/types';
  import { asksAddress, openFind } from '../lib/features';

  let path = $state(location.pathname);
  /** A note open beside the page (a wide screen), or none. */
  let item = $state<PaneT | null>(null);
  let wide = $state(true);
  /// The backup loop's word on vaults not leaving this device (for the rail's sync area).
  let unsent = $state('');
  let backupNote = $state('');
  /// **⋯ → Bring into another note…** (the parity list: it did nothing on the computer's pages): the
  /// passage waits here while a note is chosen, then goes in through `put`, open or not.
  let bringing = $state<Piece[] | null>(null);
  async function placeBrought(noteId: string | null) {
    const pieces = bringing ?? [];
    bringing = null;
    if (!pieces.length) return;
    try {
      const to: ipc.PutTo = noteId ? { note: noteId } : { new: { title: '' } };
      const r = await ipc.put(to, piecesMarkdown(pieces));
      openItem(r.id);
    } catch (e) {
      backupNote = `It could not be brought in: ${String(e)}`;
    }
  }

  /** Go somewhere inside formicaria's pages without leaving the tab; anything else as a link. */
  function go(url: string) {
    // **Home pressed on Home** goes to today's entry (Home, 2026-11-23; `HomeView` listens).
    if (url === '/f/home' && location.pathname === '/f/home' && !item) {
      dispatchEvent(new CustomEvent('fm-home-again'));
      return;
    }
    if (url.startsWith('/f/') && url !== location.pathname) {
      history.pushState(null, '', url);
      path = location.pathname;
      item = null;
    } else if (!url.startsWith('/f/')) {
      location.assign(url);
    }
  }
  /** A note: beside the page where there is room, else in its place. */
  function openItem(id: string) {
    if (wide) item = newPane('note', { noteId: id });
    else go(`/f/note/${id}`);
  }
  async function newIn(s: Section) {
    if (asksAddress(s)) return openFind();
    const props = (s.new?.props ?? {}) as Record<string, string | number>;
    const [made] = await ipc.createNotes([{ title: '', body: '', tags: [], start: '', props }]);
    if (made) openItem(made.id);
  }
  onMount(() => {
    const pop = () => {
      path = location.pathname;
      item = null;
    };
    const mq = matchMedia('(min-width: 1100px)');
    const size = () => (wide = mq.matches);
    size();
    mq.addEventListener('change', size);
    addEventListener('popstate', pop);
    // **Redo** (the parity list): Ctrl+Shift+Z / ⌘⇧Z outside the text, as the older frame has it.
    const redo = (e: KeyboardEvent) => {
      // **Alt+Home goes Home** from any page (the browser's own home key; Home, 2026-11-23).
      if (e.altKey && !e.ctrlKey && !e.metaKey && e.key === 'Home') {
        e.preventDefault();
        go('/f/home');
        return;
      }
      if (!(e.ctrlKey || e.metaKey) || !e.shiftKey || e.key.toLowerCase() !== 'z') return;
      const t = e.target as HTMLElement | null;
      if (
        t &&
        (t.isContentEditable ||
          /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) ||
          t.closest?.('.cm-editor'))
      )
        return;
      e.preventDefault();
      void undo.redo();
    };
    addEventListener('keydown', redo);
    const bring = (e: Event) => {
      const p = (e as CustomEvent<Piece>).detail;
      if (p?.md) bringing = [...(bringing ?? []), p];
    };
    addEventListener('fm-bring-to-room', bring);
    // **The backup loop and the program's reminders, once for the whole tab** (`backupLoop.ts`).
    const stopBackup = backUpInOneTab({
      onUnsent: (u) => (unsent = unsentLabel(u)),
      whileLeading: () => {
        void syncReminders();
        return ipc.listenChanges(() => void syncReminders());
      },
      said: (vault, phase) => {
        if (phase === 'conflicts')
          backupNote = `Notes in “${vault}” came back with edits from another device in both versions: open them to merge.`;
      },
    });
    return () => {
      stopBackup();
      removeEventListener('fm-bring-to-room', bring);
      removeEventListener('keydown', redo);
      removeEventListener('popstate', pop);
      mq.removeEventListener('change', size);
    };
  });
</script>

<div class="shell" class:with-item={!!item}>
  <Rail {path} {go} onnew={(s) => void newIn(s)} onopen={openItem} {unsent} />
  <UndoDelete />
  <ReminderBanner />
  <ManualPanel />
  {#if bringing}
    <ShareSheet
      files={[]}
      texts={bringing.map((p) => ({ text: p.md, subject: p.source?.title ?? '' }))}
      refused={[]}
      openNoteId={item?.noteId ?? null}
      onchoose={(id) => void placeBrought(id)}
      onclose={() => (bringing = null)}
    />
  {/if}
  {#if backupNote}
    <p class="backup-note" role="status">
      {backupNote}
      <button type="button" aria-label="dismiss" onclick={() => (backupNote = '')}>✕</button>
    </p>
  {/if}
  <main class="page">
    {#key path}
      <Feature {path} {go} onitem={openItem} />
    {/key}
  </main>
  <!-- The player, outside the page: a song keeps playing while you move between places (the
       parity list: it was the older frame's only). -->
  <div class="player"><PlayerBar onopen={openItem} /></div>
  {#if item}
    <aside class="item" aria-label="the note beside">
      <Pane
        pane={item}
        index={1}
        cols={1}
        feed={undefined}
        statuses={[]}
        shown={() => true}
        focused={true}
        startEditing={false}
        vaults={[]}
        onopen={openItem}
        onresolve={async () => {}}
        onmove={() => {}}
        onstatus={() => {}}
        onnavigate={openItem}
        onsaved={() => {}}
        onchange={(patch) => item && (item = { ...item, ...patch })}
        ontitle={() => {}}
        onreorder={() => {}}
        onresize={() => {}}
        onclose={() => (item = null)}
        onfocus={() => {}}
        headed={false}
      />
    </aside>
  {/if}
</div>

<style>
  .shell {
    display: flex;
    height: 100dvh;
    overflow: hidden;
  }
  .page {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  .backup-note {
    position: fixed;
    top: var(--space-2);
    left: 50%;
    translate: -50% 0;
    z-index: 60;
    margin: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--warning-bg, #fff4d6);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
  }
  .player {
    position: fixed;
    left: 14.5rem;
    right: 0;
    bottom: 0;
    z-index: 40;
    pointer-events: none;
  }
  .player :global(*) {
    pointer-events: auto;
  }
  .item {
    flex: 0 0 min(40rem, 42vw);
    border-left: 1px solid var(--border);
    background: var(--surface);
    overflow: auto;
  }
</style>

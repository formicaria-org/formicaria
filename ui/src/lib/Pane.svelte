<script lang="ts">
  // One pane of the workspace: a compact header (view picker, per-view controls, close) over
  // one of the existing renderers. The renderers are pure and already `height:100%`, so a pane
  // is a thin frame around them — the whole reason the flexible grid is cheap.
  import {
    draggable,
    dropTargetForElements,
  } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import Board from '../renderers/Board.svelte';
  import Agenda from '../renderers/Agenda.svelte';
  import Calendar from '../renderers/Calendar.svelte';
  import Timeline from '../renderers/Timeline.svelte';
  import Search from '../renderers/Search.svelte';
  import Activity from '../renderers/Activity.svelte';
  import Discussions from '../renderers/Discussions.svelte';
  import Icon from './Icon.svelte';
  import VaultBadge from './VaultBadge.svelte';
  import ProposalReview from './ProposalReview.svelte';
  import type { Pane, PaneKind, Feed } from './panes';
  import { paneTitle, clampSpan, BUILTIN_PANES, rendererKind } from './panes';
  import type { ObjectMeta, ViewInfo, Board as BoardT, ConflictInfo } from './types';
  import { orderColumns, moveValue } from './boardOrder';

  interface Props {
    pane: Pane;
    feed: Feed | undefined; // the fetched data for this pane's feed (undefined = loading)
    index: number; // this pane's slot in the workspace (drag source/target identity)
    cols: number; // the workspace column count (clamps a resize)
    statuses: string[];
    savedViews: ViewInfo[];
    shown: (n: { id: string; vault: string }) => boolean;
    focused: boolean;
    startEditing: boolean; // a note pane opened via "New note" starts in the editor
    vaults: string[]; // all vault names — a note pane offers "Copy to" the others
    onopen: (id: string) => void;
    /// Resolve a conflict by keeping a side — only reachable for the kinds with no markers to edit.
    onresolve: (c: ConflictInfo, keep: 'theirs' | 'mine' | 'edited') => Promise<void>;
    onmove: (groupBy: string, id: string, value: string, beforeId: string | null) => void;
    onstatus: (id: string, value: string | null) => void;
    onnavigate: (id: string) => void; // a note pane's chip was followed → open that note
    onsaved: () => void; // a note pane wrote → schedule the git commit
    onchange: (patch: Partial<Pane>) => void;
    onreorder: (from: number, to: number) => void;
    onresize: (patch: { colSpan?: number; rowSpan?: number }) => void;
    onclose: () => void;
    onfocus: () => void;
    /// Keep this arrangement as a named view. Only the kinds that *are* an arrangement offer it.
    onsaveview?: () => void;
    ondeleteview?: () => void;
  }
  let {
    pane,
    feed,
    index,
    cols,
    statuses,
    savedViews,
    shown,
    focused,
    startEditing,
    vaults,
    onopen,
    onresolve,
    onmove,
    onstatus,
    onnavigate,
    onsaved,
    onchange,
    onreorder,
    onresize,
    onclose,
    onfocus,
    onsaveview,
    ondeleteview,
  }: Props = $props();

  // A note pane can "maximize" to fill the workspace (the old full-screen toggle, repurposed
  // as a span change). NotePanel's ⤢/⤡ button drives it; `wide` picks the icon.
  const maximized = $derived(pane.colSpan >= cols);
  function toggleMax() {
    onresize(maximized ? { colSpan: 1, rowSpan: 1 } : { colSpan: cols, rowSpan: 2 });
  }

  let dragover = $state(false);

  // The header grip is the drag handle; the whole pane is the drop target. A pane drag carries
  // its `paneIndex`; card/column drags (from a Board inside a pane) carry `id`/`columnValue`, so
  // the branches never cross — a card dropped on a pane is ignored here, a pane dropped on a
  // board column is ignored there. Same svelte-action shape as Board's column dnd.
  function gripDrag(node: HTMLElement, i: number) {
    let current = i;
    const cleanup = draggable({ element: node, getInitialData: () => ({ paneIndex: current }) });
    return { update: (v: number) => (current = v), destroy: cleanup };
  }
  function paneDrop(node: HTMLElement, i: number) {
    let current = i;
    const cleanup = dropTargetForElements({
      element: node,
      canDrop: ({ source }) => typeof source.data.paneIndex === 'number',
      onDragEnter: () => (dragover = true),
      onDragLeave: () => (dragover = false),
      onDrop: ({ source }) => {
        dragover = false;
        const from = source.data.paneIndex;
        if (typeof from === 'number' && from !== current) onreorder(from, current);
      },
    });
    return { update: (v: number) => (current = v), destroy: cleanup };
  }

  // Corner-drag resize: measure a single track from the pane's own rendered box (width /
  // current colSpan), then set the span to whatever the pointer now covers. Self-measured —
  // no access to the grid container needed — and clamped to the column count. Pointer capture
  // keeps the drag alive even as the pointer leaves the small grip.
  let paneEl = $state<HTMLElement | undefined>(undefined);
  function startResize(e: PointerEvent) {
    if (!paneEl) return;
    e.preventDefault();
    e.stopPropagation();
    const grip = e.currentTarget as HTMLElement;
    grip.setPointerCapture(e.pointerId);
    const rect = paneEl.getBoundingClientRect();
    const colW = rect.width / Math.max(1, pane.colSpan);
    const rowH = rect.height / Math.max(1, pane.rowSpan);
    const move = (ev: PointerEvent) => {
      const colSpan = clampSpan(Math.round((ev.clientX - rect.left) / colW), cols);
      const rowSpan = Math.max(1, Math.min(Math.round((ev.clientY - rect.top) / rowH), 4));
      if (colSpan !== pane.colSpan || rowSpan !== pane.rowSpan) onresize({ colSpan, rowSpan });
    };
    const up = (ev: PointerEvent) => {
      grip.releasePointerCapture(ev.pointerId);
      grip.removeEventListener('pointermove', move);
      grip.removeEventListener('pointerup', up);
    };
    grip.addEventListener('pointermove', move);
    grip.addEventListener('pointerup', up);
  }

  // Cards this pane shows, after the global vault filter.
  const cards = $derived((feed?.cards ?? []).filter(shown));
  // Conflicted notes for the Collaboration surface, vault-filtered like the cards (the filter reads
  // the nested note, since a conflict now carries its kind alongside it).
  const conflictNotes = $derived((feed?.conflicts ?? []).filter((c) => shown(c.note)));
  /// Which conflict rows have a resolution in flight, so a double-tap cannot fire two resolutions at
  /// the same path (the second would fail with "not in conflict" and read as a broken button).
  let resolving = $state<Record<string, boolean>>({});
  async function keep(c: ConflictInfo, side: 'theirs' | 'mine' | 'edited') {
    if (resolving[c.path]) return;
    resolving = { ...resolving, [c.path]: true };
    try {
      await onresolve(c, side);
    } finally {
      resolving = { ...resolving, [c.path]: false };
    }
  }
  // Which proposals have their diff expanded — so a proposal's diff is fetched only when opened,
  // not once per proposal on render.
  let openProposal = $state<Record<string, boolean>>({});
  // A board with its cards vault-filtered (columns kept, even if emptied).
  const board = $derived(
    feed?.board
      ? { ...feed.board, columns: feed.board.columns.map((c) => ({ ...c, cards: c.cards.filter(shown) })) }
      : null,
  );

  // The picker's options: the built-in kinds (from the one registry, so a new kind appears here,
  // in the ⌘K palette, and in the bottom bar together) plus each saved `.view` (by name).
  const KINDS: { value: string; label: string }[] = BUILTIN_PANES.map((b) => ({
    value: b.kind,
    label: b.label,
  }));

  function pick(value: string) {
    if (value.startsWith('view:')) {
      onchange({ kind: 'view', viewName: value.slice(5) });
    } else {
      onchange({ kind: value as PaneKind, viewName: null });
    }
  }
  const pickValue = $derived(pane.kind === 'view' ? `view:${pane.viewName}` : pane.kind);

  // The view picker is a *rotator*, not a dropdown: the same options in a ring the user steps
  // through by clicking (forward) or scrolling the wheel over it (either direction). Same
  // `pick()` write-back as the old <select>, just a different way to reach a value.
  const options = $derived([
    ...KINDS,
    ...savedViews.filter((v) => !v.error).map((v) => ({ value: `view:${v.name}`, label: v.name })),
  ]);
  const currentIndex = $derived(Math.max(0, options.findIndex((o) => o.value === pickValue)));
  const currentLabel = $derived(options[currentIndex]?.label ?? 'Board');

  /// **What the saved view on show leaves out** — so the header can say so.
  ///
  /// A `view: board` draws through the very same `Board.svelte` as the Board pane, so a view whose
  /// filter removes a column removes it with nothing on screen to explain the gap. That is not
  /// hypothetical: the owner's only `.view` hides one status, and the missing column read as
  /// missing notes (2026-08-24).
  ///
  /// **Read from the feed, not from `savedViews`.** The words belong to the payload they describe:
  /// they arrive with the board itself, from the same parse that produced it. Looking them up in
  /// the view *list* would make the explanation depend on a second fetch that happens once per
  /// vault change and has failed outright on the phone — and a filtered board with no explanation
  /// is precisely the bug. `undefined` until the feed lands, which is also when the board lands.
  const shownView = $derived(pane.kind === 'view' ? feed?.view : undefined);
  const hides = $derived(shownView?.filters ?? []);
  /// The whole sentence, including the way out — the visible chip is one line and gets clipped in a
  /// narrow pane, so the accessible name has to be the complete thought on its own.
  const hidesTitle = $derived(
    `“${pane.viewName}” shows only notes where ${hides.join(', and ')}. Click to see everything.`,
  );
  /// Leave the view for the built-in renderer it shadows, unfiltered, keeping its grouping — the
  /// one click that answers "where did my column go". The rotator walks back to the view.
  function showEverything() {
    if (!shownView) return;
    onchange({
      kind: rendererKind(shownView.renderer),
      viewName: null,
      groupBy: shownView.group_by ?? pane.groupBy,
    });
  }
  /// **Two steps, not a modal.** Deleting a view removes a file from someone's vault, so it must
  /// not ride on one stray click — but a modal is a whole new surface for something done twice a
  /// year, and this app already decided that a rare action belongs inline (the same reasoning that
  /// removed the command palette). The second click is the confirmation.
  let confirmDelete = $state(false);
  /// Disarm when the pane starts showing something else, or the armed button follows you onto a
  /// different view and the next click deletes *that* one.
  $effect(() => {
    void pane.viewName;
    confirmDelete = false;
  });
  function rotate(dir: number) {
    if (!options.length) return;
    pick(options[(currentIndex + dir + options.length) % options.length].value);
  }

  /// **The whole header rotates the view, not just the little label.**
  ///
  /// The rotator button worked and nobody found it — it is one small target among the header's
  /// controls, and nothing about it says "scroll me". The header is the biggest thing in a pane
  /// that is not content, it is already the drag handle, and it is where you look when you are
  /// thinking about *this window* rather than what is in it. So the wheel anywhere across it
  /// spins the view, and a horizontal swipe does the same on a touch screen.
  ///
  /// **Not the pane body.** A board scrolls horizontally by design and the agenda scrolls
  /// vertically; stealing those gestures from the content would trade one discoverable action
  /// for two broken ones.
  ///
  /// A note pane has no view to rotate, so it is exempt — `options` describes ways of looking at
  /// a *collection*, and a note is one document.
  const rotatable = $derived(pane.kind !== 'note');

  /// **One gesture is one step**, and the rule depends only on *how far* you scrolled.
  ///
  /// Two bugs came out of this control, both reported within minutes, and both were the same
  /// mistake in different clothes: the amount a step cost depended on **history** rather than on
  /// the gesture in front of it.
  ///
  /// 1. Rotating on every event — a wheel sends a burst per detent and a trackpad sends a stream
  ///    plus inertia, so a light movement span several views.
  /// 2. Then the cooldown, when it blocked a step, was *cleared by a reversal* but not by
  ///    continuing. So after any step the direction you were already going was throttled and the
  ///    opposite fired instantly: "rolling up seems to be sensitive differently than rolling
  ///    down". It was, by construction.
  ///
  /// So there is no direction memory here at all any more, and nothing carries between gestures.
  /// A wheel event is judged on its own size:
  ///
  /// - **A detent** — one click of a real wheel — turns the view once, at once, either way. You
  ///   cannot produce these faster than your fingers move, so throttling them only ever makes a
  ///   deliberate act feel ignored.
  /// - **Smooth scrolling** — a trackpad, or a high-resolution wheel — accumulates, because its
  ///   events are far smaller and far more numerous. Only there is a cooldown needed, and it
  ///   applies the same in both directions.

  /// Above this, a single event is a discrete detent. Chrome reports 100–120 for one, Firefox
  /// three lines (120 normalised); a trackpad's individual deltas are an order of magnitude less.
  const WHEEL_NOTCH = 50;
  /// How much smooth scrolling makes one step. Deliberately several times a detent: these arrive
  /// in long streams, and the original complaint was a barely-there movement spending views.
  const WHEEL_SMOOTH_STEP = 180;
  /// Only for smooth scrolling, and applied identically whichever way you are going — it exists
  /// to stop inertia banking steps after your fingers have left the surface, nothing else.
  const WHEEL_COOLDOWN = 220;
  /// Quiet long enough to end a gesture, so nothing you scrolled a moment ago is still owed.
  const WHEEL_GAP = 350;

  let wheelAccum = 0;
  let lastWheelAt = 0;
  let lastRotateAt = 0;

  /// A wheel delta in pixels, whichever unit the browser chose to report.
  ///
  /// `deltaMode` is lines on Firefox and pages in some configurations, so the raw number means
  /// nothing on its own. A line counts as 40px rather than the ~16 it really is, because Firefox
  /// sends **three** lines per detent — the number that matters is the one that makes a detent
  /// weigh the same in both browsers.
  function wheelPixels(e: WheelEvent): number {
    const scale = e.deltaMode === 1 ? 40 : e.deltaMode === 2 ? 800 : 1;
    return e.deltaY * scale;
  }

  function headWheel(e: WheelEvent) {
    if (!rotatable) return;
    // Only when the wheel is actually being turned vertically: a trackpad's horizontal flick is
    // how you scroll a board, and it reaches here when the pointer is over the header.
    if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
    e.preventDefault();

    const now = performance.now();
    const dy = wheelPixels(e);
    const dir = dy > 0 ? 1 : -1;

    // A detent: unambiguous, deliberate, and answered immediately in either direction.
    if (Math.abs(dy) >= WHEEL_NOTCH) {
      wheelAccum = 0;
      lastWheelAt = now;
      lastRotateAt = now;
      rotate(dir);
      return;
    }

    // Smooth scrolling: add up, and start over after a pause so nothing is carried into the
    // next gesture.
    if (now - lastWheelAt > WHEEL_GAP) wheelAccum = 0;
    lastWheelAt = now;
    wheelAccum += dy;
    if (Math.abs(wheelAccum) < WHEEL_SMOOTH_STEP) return;
    // Discarded rather than held: holding it at the threshold is what pre-charged one direction
    // last time. Either way, a step always costs a whole gesture.
    wheelAccum = 0;
    if (now - lastRotateAt < WHEEL_COOLDOWN) return;
    lastRotateAt = now;
    rotate(dir);
  }


  // Swipe: recorded on the header only, and deliberately generous about what counts as one.
  let touchX = 0;
  let touchY = 0;
  /** Below this a swipe is a tap that wandered; ~1/8 of a phone's width. */
  const SWIPE_MIN = 48;
  function headTouchStart(e: TouchEvent) {
    const t = e.changedTouches[0];
    touchX = t?.clientX ?? 0;
    touchY = t?.clientY ?? 0;
  }
  function headTouchEnd(e: TouchEvent) {
    if (!rotatable) return;
    const t = e.changedTouches[0];
    if (!t) return;
    const dx = t.clientX - touchX;
    const dy = t.clientY - touchY;
    // Horizontal intent, not a vertical scroll that started on the header.
    if (Math.abs(dx) < SWIPE_MIN || Math.abs(dx) <= Math.abs(dy)) return;
    // Swipe left = forward, matching every carousel: the content moves the way your finger did.
    rotate(dx < 0 ? 1 : -1);
  }

  // **Column order, reconnected.** `boardOrder.ts` is a pure, tested core — and after the
  // pane rewrite nothing imported it: `onreorder` was `() => {}`, so dragging a column
  // header did nothing while the drag still started and the cursor still said `grab`. An
  // affordance that looks live and is dead is worse than no affordance.
  //
  // Keyed by **this pane's `groupBy`**, because the order of "todo/doing/done" has nothing
  // to say about the order of "project A/project B". Two panes grouped the same way share
  // one order, which is right — it is the same board seen twice.
  //
  // A view preference, so `localStorage` and never a vault: what you are currently looking
  // at is not knowledge.
  const ORDER_KEY = 'fm-board-order';

  function allOrders(): Record<string, string[]> {
    try {
      return JSON.parse(localStorage.getItem(ORDER_KEY) ?? '{}');
    } catch {
      return {};
    }
  }

  // Bumped on write so the `$derived` below re-runs — localStorage is not reactive.
  let orderVersion = $state(0);

  function ordered(b: BoardT): BoardT {
    void orderVersion;
    const saved = allOrders()[pane.groupBy];
    return saved?.length ? { ...b, columns: orderColumns(b.columns, saved) } : b;
  }

  function reorderColumns(fromValue: string, toValue: string, before: boolean) {
    if (!board) return;
    // Move within the order the user is *looking at*, not the server's — otherwise the
    // second drag is computed against a list that is no longer on screen. `moveValue`
    // returns the whole permutation, so the first drag persists a complete, stable order.
    const visible = ordered(board).columns.map((c) => c.value);
    const next = moveValue(visible, fromValue, toValue, before);
    try {
      localStorage.setItem(ORDER_KEY, JSON.stringify({ ...allOrders(), [pane.groupBy]: next }));
      orderVersion++;
    } catch {
      /* private mode — the order just won't persist */
    }
  }
</script>

<section
  class="pane"
  class:focused
  class:dragover
  bind:this={paneEl}
  use:paneDrop={index}
  onpointerdowncapture={onfocus}
  aria-label={paneTitle(pane)}
>
  <!-- The whole header is the drag handle (not just the grip) — grab anywhere on the top bar to
       move the pane. Clicks on the controls inside still register (a click is not a drag). -->
  <header
    class="pane-head"
    class:grip-only={pane.kind === 'note'}
    use:gripDrag={index}
    onwheel={headWheel}
    ontouchstart={headTouchStart}
    ontouchend={headTouchEnd}
    role="toolbar"
    tabindex="-1"
    aria-label="pane controls — drag to rearrange, scroll or swipe to change the view"
    title={rotatable
      ? 'Drag to rearrange · scroll or swipe here to change the view'
      : 'Drag to rearrange this pane'}
  >
    <span class="grip" aria-hidden="true">
      <Icon name="grip" size={13} />
    </span>
    <!-- A note pane wears NotePanel's own chrome (title, status, edit, delete, close), so the
         pane header shrinks to just the drag grip. Every other kind gets the view picker. -->
    {#if pane.kind !== 'note'}
      <!-- A rotator, not a dropdown: click to advance, scroll the wheel to spin either way. -->
      <button
        type="button"
        class="kind rotator"
        onclick={() => rotate(1)}
        title="Click, or scroll/swipe anywhere on this bar, to change the view"
        aria-label="pane view"
      >
        {currentLabel}
      </button>

      <!-- Per-view controls, inline in the pane header (each pane tunes itself). -->
      {#if pane.kind === 'board'}
        <input
          class="ctl group"
          list="pane-props"
          value={pane.groupBy}
          oninput={(e) => onchange({ groupBy: (e.currentTarget as HTMLInputElement).value })}
          spellcheck="false"
          title="group by"
          aria-label="group by"
        />
      {:else if pane.kind === 'agenda'}
        <div class="seg">
          <button class:on={pane.agendaMode === 'month'} onclick={() => onchange({ agendaMode: 'month' })}>M</button>
          <button class:on={pane.agendaMode === 'week'} onclick={() => onchange({ agendaMode: 'week' })}>W</button>
          <button class:on={pane.agendaMode === 'list'} onclick={() => onchange({ agendaMode: 'list' })}>L</button>
        </div>
      {:else if pane.kind === 'view' && hides.length}
        <!-- **A filtered view has to admit it.** Everything else in this chain tunes a pane; this
             one explains it. The words come from the server (`ViewInfo.filters`), so nothing here
             knows what a status is and a view that narrows by tag or date reads just as well.
             The text is *visible*, not tucked into `title`: a phone has no hover, and the whole
             point is to be legible at the moment the column looks missing. -->
        <button type="button" class="hides" onclick={showEverything} title={hidesTitle} aria-label={hidesTitle}>
          filtered: {hides.join(' · ')}
        </button>
      {:else if pane.kind === 'search'}
        <input
          class="ctl"
          type="search"
          placeholder="Search…"
          value={pane.query}
          oninput={(e) => onchange({ query: (e.currentTarget as HTMLInputElement).value })}
          spellcheck="false"
          aria-label="search"
        />
      {/if}
      <!-- **Keeping an arrangement had no button at all.** `newView` is a command with an empty
           default key and the palette that used to carry it is gone, so "New view" was reachable
           only by binding a key to it in Settings — a labelled capability with no way in, which is
           the failure `outstanding.md` §2.6b is about. Offered on the three kinds that *are* an
           arrangement; a note, a search or an existing view is not one. -->
      {#if onsaveview && (pane.kind === 'board' || pane.kind === 'agenda' || pane.kind === 'timeline')}
        <button
          type="button"
          class="ctl save-view-btn"
          onclick={onsaveview}
          title="Keep this arrangement as a named view"
          aria-label="save this view">Save view</button>
      {/if}
      <!-- The other half of the same decision. `delete_view` has existed end to end — command,
           `ipc.ts`, mock — since views became saveable, with no button anywhere calling it, so a
           view could be made from the app and then only removed with a file manager. -->
      {#if ondeleteview && pane.kind === 'view' && pane.viewName}
        <button
          type="button"
          class="ctl save-view-btn"
          class:armed={confirmDelete}
          onclick={() => {
            if (confirmDelete) {
              confirmDelete = false;
              ondeleteview();
            } else {
              confirmDelete = true;
            }
          }}
          onblur={() => (confirmDelete = false)}
          title={confirmDelete
            ? 'Click again to delete this view. The notes it showed are not touched.'
            : 'Delete this saved view'}
          aria-label={confirmDelete ? 'confirm deleting this view' : 'delete this view'}
          >{confirmDelete ? 'Delete?' : 'Delete view'}</button>
      {/if}
    {/if}

    <span class="spacer"></span>
    {#if pane.kind !== 'note'}
      <button class="pane-close" onclick={onclose} aria-label="close pane" title="Close pane">
        <Icon name="close" size={14} />
      </button>
    {/if}
  </header>

  <div class="pane-body" class:note-body={pane.kind === 'note'}>
    {#if pane.kind === 'note'}
      {#if pane.noteId}
        {#await import('./NotePanel.svelte') then { default: NotePanel }}
          <NotePanel
            id={pane.noteId}
            {statuses}
            {startEditing}
            {vaults}
            solo
            wide={maximized}
            onclose={onclose}
            onnavigate={onnavigate}
            onsaved={onsaved}
            ontogglewide={toggleMax}
          />
        {/await}
      {:else}
        <p class="pane-empty">No note.</p>
      {/if}
    {:else if pane.kind === 'board' || (pane.kind === 'view' && board)}
      {#if board}
        <Board
          board={ordered(board)}
          onmove={(id, value, beforeId) => onmove(pane.groupBy, id, value, beforeId)}
          onreorder={reorderColumns}
          {onopen}
          {statuses}
          onstatus={onstatus}
        />
      {:else}
        <p class="pane-empty">Loading…</p>
      {/if}
    {:else if pane.kind === 'search'}
      <Search {cards} query={pane.query} {onopen} />
      <!-- A saved view asking for the flat list. Until now `renderer: search` parsed, was accepted
           by the loader, and then drew as a Timeline — the one place the "a broken view names
           itself, never vanishes" discipline was not applied, because nothing was broken: it just
           silently drew something else. No `query` prop: this is a filter someone wrote, not a
           search someone typed (see the comment in `Search.svelte`). -->
    {:else if pane.kind === 'view' && shownView?.renderer === 'search'}
      <Search {cards} {onopen} />
    {:else if pane.kind === 'activity'}
      <Activity {shown} {onopen} />
    {:else if pane.kind === 'discussions'}
      <Discussions discussions={feed?.discussions ?? []} {shown} {onopen} />
    {:else if pane.kind === 'collaboration'}
      <!-- Two things that need a person: conflicts (urgent — an unresolved merge blocks every
           commit) surface first, then proposals through the usual Timeline. -->
      {#if conflictNotes.length}
        <section class="conflicts-feed">
          <h3 class="conflicts-head">⚠ Needs resolution</h3>
          <ul>
            {#each conflictNotes as c (c.path || c.note.id)}
              <li>
                <!-- Two shapes, because there are two kinds of conflict and only one of them can be
                     resolved by editing the note. Telling a user to "open it and keep the text you
                     want" when one side deleted the file is advice that cannot be followed — it is
                     what left a vault frozen for a week with nothing to click. -->
                <button class="conflict-row" onclick={() => onopen(c.note.id)}>
                  <VaultBadge vault={c.vault} />
                  <span class="conflict-title">{c.note.title ?? c.note.preview}</span>
                  <span class="conflict-tag">{c.has_markers ? 'in the text' : 'pick a side'}</span>
                </button>
                <p class="conflict-what">{c.what}</p>
                {#if c.has_markers}
                  <!-- **A marker conflict needs an explicit "I'm done".** Editing the note settles
                       nothing as far as git is concerned — the index keeps all three stages, so the
                       vault stays frozen while the note quietly stops looking conflicted. This is the
                       button that stages the reconciliation; it refuses while markers remain, because
                       staging a marked-up file is what git reads as "resolved" and would publish
                       `<<<<<<<` as the note's content. -->
                  <div class="conflict-actions">
                    <button type="button" onclick={() => onopen(c.note.id)}>Open the note</button>
                    <button
                      type="button"
                      disabled={resolving[c.path]}
                      onclick={() => keep(c, 'edited')}
                    >
                      Mark resolved
                    </button>
                  </div>
                {:else}
                  <div class="conflict-actions">
                    <button
                      type="button"
                      disabled={resolving[c.path]}
                      onclick={() => keep(c, 'theirs')}
                    >
                      Keep the other device's version
                    </button>
                    <button
                      type="button"
                      disabled={resolving[c.path]}
                      onclick={() => keep(c, 'mine')}
                    >
                      Keep this device's version
                    </button>
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      <!-- Proposals: each expands to its diff (fetched lazily on open); "open" jumps to the
           proposal note itself, where its discussion lives. -->
      {#if cards.length}
        <section class="proposals-feed">
          <h3 class="proposals-head">Proposals</h3>
          <ul>
            {#each cards as p (p.id)}
              <li>
                <details bind:open={openProposal[p.id]}>
                  <summary class="proposal-row">
                    <VaultBadge vault={p.vault} />
                    <span class="proposal-title">{p.title ?? p.preview}</span>
                    <button
                      class="proposal-open"
                      onclick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        onopen(p.id);
                      }}
                      title="Open the proposal note and its discussion">open</button
                    >
                  </summary>
                  {#if openProposal[p.id]}
                    <ProposalReview id={p.id} />
                  {/if}
                </details>
              </li>
            {/each}
          </ul>
        </section>
      {:else}
        <p class="no-proposals">No open proposals.</p>
      {/if}
    {:else if pane.kind === 'agenda'}
      {#if pane.agendaMode === 'list'}
        <Agenda {cards} {onopen} />
      {:else}
        <Calendar {cards} {onopen} range={pane.agendaMode === 'week' ? 'week' : 'month'} />
      {/if}
    {:else}
      <!-- timeline, or a flat-list saved view -->
      <Timeline {cards} {onopen} {statuses} onstatus={onstatus} />
    {/if}
  </div>

  <!-- Corner resize handle: drag to change how many columns/rows this pane spans. -->
  <span
    class="resize-grip"
    role="slider"
    tabindex="-1"
    aria-label="resize pane"
    aria-valuenow={pane.colSpan}
    title="Drag to resize"
    onpointerdown={startResize}
  ></span>
</section>

<datalist id="pane-props">
  <option value="status"></option>
  <option value="project"></option>
  <option value="tags"></option>
</datalist>

<style>
  .pane {
    /* Renderers inside a pane must size to *the pane*, not the window. A colSpan:1 pane in a
       4-column workspace on a wide monitor is ~380px, and viewport media queries were handing
       it desktop-width board columns it could not fit; a maximised pane on a small window got
       `85vw` columns inside a box that was not 85vw. The media queries were asking the wrong
       element. This is what lets one renderer be correct at any size — and it is the capability
       that did not exist when the standard advice was "build a separate mobile site". */
    container-type: inline-size;
    container-name: pane;

    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .pane.focused {
    border-color: var(--accent);
  }
  /* Drop affordance while another pane is dragged over this one. */
  .pane.dragover {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .grip {
    display: inline-flex;
    align-items: center;
    flex: none;
    color: var(--text-muted);
    cursor: grab;
    touch-action: none;
  }
  .grip:active {
    cursor: grabbing;
  }
  /* A small triangular handle in the bottom-right corner; drag it to resize. */
  .resize-grip {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 16px;
    height: 16px;
    cursor: nwse-resize;
    touch-action: none;
    z-index: 2;
    background: linear-gradient(
      135deg,
      transparent 0 50%,
      var(--border) 50% 62%,
      transparent 62% 74%,
      var(--border) 74% 86%,
      transparent 86%
    );
  }
  .resize-grip:hover {
    background: linear-gradient(
      135deg,
      transparent 0 50%,
      var(--accent) 50% 62%,
      transparent 62% 74%,
      var(--accent) 74% 86%,
      transparent 86%
    );
  }
  .pane-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex: none;
    cursor: grab; /* the whole header is the drag handle */
    user-select: none;
    touch-action: none;
  }
  .pane-head:active {
    cursor: grabbing;
  }
  /* The interactive controls sit above the header's grab cursor — they keep their own. */
  .pane-head .rotator,
  .pane-head .seg button,
  .pane-head .pane-close {
    cursor: pointer;
  }
  .pane-head .ctl {
    cursor: auto;
  }
  .kind {
    font: inherit;
    font-size: 0.85rem;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 4px;
  }
  .rotator {
    min-width: 5rem;
    text-align: left;
    white-space: nowrap;
  }
  /* Sized to the header rather than to its text: the sentence can be long (several filter
     entries), and a chip that pushed the close button off the end would trade one lost control
     for another. It clips, and `title`/`aria-label` carry the whole thing. */
  .hides {
    font: inherit;
    font-size: 0.85rem;
    min-width: 0;
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--surface);
    color: var(--muted);
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    cursor: pointer;
  }
  .hides:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .rotator:hover {
    border-color: var(--accent);
  }
  .save-view-btn {
    cursor: pointer;
    white-space: nowrap;
  }
  /* Armed for the second click. Colour alone would be the only signal for a reader who cannot see
     it, so the label changes too ("Delete view" → "Delete?") and the accessible name with it. */
  .save-view-btn.armed {
    background: var(--danger-bg);
    color: var(--danger-fg);
    border-color: var(--danger-fg);
  }
  .ctl {
    font: inherit;
    font-size: 0.85rem;
    min-width: 0;
    width: 8rem;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .seg button {
    font: inherit;
    font-size: 0.8rem;
    padding: 2px 7px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .spacer {
    flex: 1;
  }
  .pane-close {
    flex: none;
    display: inline-flex;
    align-items: center;
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .pane-close:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* A note pane hands its whole body to NotePanel, which scrolls itself (height:100%,
     width:100%). No pane-level scroll, no padding — the note owns the space. */
  .pane-body.note-body {
    display: flex;
    overflow: hidden;
    padding: 0;
  }
  .pane-body.note-body > :global(.panel) {
    box-shadow: none;
  }
  .pane-empty {
    padding: var(--space-4);
    color: var(--text-muted);
  }
  /* A conflict's own sentence: what the two sides did, in plain words, under the row it belongs to. */
  .conflict-what {
    margin: 0 0 0.4rem 0.5rem;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .conflict-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin: 0 0 0.75rem 0.5rem;
  }
  .conflict-actions button {
    /* Its own sizing, not `.icon-btn`: every `.icon-btn` in the top bar is hidden in the narrow
       layouts, which is how a control that must stay reachable on a phone silently vanishes. */
    min-height: 2.5rem;
    padding: 0 0.85rem;
    border-radius: 0.4rem;
    border: 1px solid var(--border);
    background: var(--surface);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .conflict-actions button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  /* Conflicts sit above the proposals in the Collaboration surface — urgent, so accent-framed. */
  .conflicts-feed {
    margin: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .conflicts-head {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    color: var(--accent);
  }
  .conflicts-feed ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .conflict-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .conflict-row:hover {
    background: var(--surface-hover);
  }
  .conflict-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .conflict-tag {
    flex: none;
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--accent);
  }

  .proposals-feed {
    margin: var(--space-2);
  }
  .proposals-head {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .proposals-feed ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .proposals-feed li {
    border-bottom: 1px solid var(--border, rgba(127, 127, 127, 0.18));
  }
  .proposal-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    list-style: none;
  }
  .proposal-row::-webkit-details-marker {
    display: none;
  }
  .proposal-row:hover {
    background: var(--surface-hover);
  }
  .proposal-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proposal-open {
    flex: none;
    font: inherit;
    font-size: var(--text-xs);
    padding: 0 var(--space-1);
    background: none;
    border: 1px solid var(--border, rgba(127, 127, 127, 0.3));
    border-radius: var(--radius-sm);
    color: var(--muted);
    cursor: pointer;
  }
  .no-proposals {
    margin: var(--space-3);
    color: var(--muted);
    font-size: var(--text-sm);
  }
</style>

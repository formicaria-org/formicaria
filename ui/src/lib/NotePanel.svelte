<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import {
    getNote,
    updateBody,
    setProperty,
    deleteNote,
    copyNote,
    copyStatus,
    uncopyNote,
    resolveAsset as ipcResolveAsset,
    assetStatus,
    assetUrl,
    streamsBlobs,
    openExternal,
    ingestFile,
    search,
    recent,
    reply as ipcReply,
    thread as ipcThread,
    backlinks as ipcBacklinks,
  } from './ipc';
  import {
    renderInto,
    type AssetFailure,
    type ResolvedAsset,
    type ResolvedNote,
    type ResolvedEmbed,
  } from './render';
  import { CALLOUT_TYPES, TEXT_TOKENS } from './render-vocab';
  import { clickOutside } from './clickOutside';
  import { parseStamp, toStamp } from './stamp';
  import { caretXY, clamp } from './caret';
  import { countOf, nthIndexOf } from './locate';
  import VaultBadge from './VaultBadge.svelte';
  import EditedBy from './EditedBy.svelte';
  import { lastEditFor } from './activity.svelte';
  import Whiteboard from './Whiteboard.svelte';
  import type { NoteDetail, ObjectMeta, ThreadMessage } from './types';

  let {
    id,
    onclose,
    onsaved,
    onnavigate,
    ontogglewide,
    wide = true,
    solo = true,
    statuses = [],
    vaults = [],
    startEditing = false,
  }: {
    id: string;
    onclose: () => void;
    onsaved?: () => void;
    /** A note chip in the read view was clicked — push it onto the trail. */
    onnavigate?: (id: string) => void;
    /** The full-screen toggle lives in this header but the state is the whole
     *  trail's, so App owns it. */
    ontogglewide?: () => void;
    /** All vault names — the "Copy to" control offers the ones that aren't this note's. */
    vaults?: string[];
    wide?: boolean;
    /** The only pane in the trail. A solo wide pane fills the viewport (the
     *  reading default); once there are siblings, panes keep their column width
     *  so the trail stays visible. */
    solo?: boolean;
    /** Known status values, for the status field's datalist (data-driven). */
    statuses?: string[];
    /** Open a fresh note straight into edit mode (the "New note" flow). */
    startEditing?: boolean;
  } = $props();

  let note = $state<NoteDetail | null>(null);
  let content = $state<HTMLElement | undefined>(undefined);
  let error = $state<string | null>(null);
  // Transient success line (e.g. after a drag-drop copy). Auto-clears.
  let notice = $state<string | null>(null);
  let editing = $state(false);

  // **Pure full-screen board.** A phone is small and Excalidraw's own floating tools already eat
  // into it, so a board can fill the *entire* viewport — nothing but the canvas, no note header, no
  // app chrome. Entering pushes a history state so the **phone's Back button** (which fires
  // `popstate`) leaves full screen instead of the app; a small floating ✕ is the on-screen twin.
  let boardFull = $state(false);
  // Excalidraw sizes to its container; nudge it after the size changes so the canvas fills the new box.
  async function nudgeResize() {
    await tick();
    window.dispatchEvent(new Event('resize'));
  }
  function enterBoardFull() {
    boardFull = true;
    try {
      history.pushState({ boardFull: true }, '');
    } catch {
      /* no history (rare) — the ✕ still exits */
    }
    void nudgeResize();
  }
  // Both the Back button and the ✕ funnel through `history.back()`, so `popstate` is the single exit
  // path — no double-pop, no dangling history entry.
  function onPopState() {
    if (boardFull) {
      boardFull = false;
      void nudgeResize();
    }
  }
  // Deleting is destructive + irreversible, so the button arms a confirm strip
  // (a second, deliberate click) rather than firing on the first press.
  let confirmingDelete = $state(false);

  // The note "options" window opened by the single `＋` in the header. Only identity — the vault
  // (audience) and who last edited — stays on the row; Edit, Copy, Delete (and the properties, via
  // Edit) live here, so a phone header stays legible and refinement is one tap away. It is a
  // *window* (a card), not a dropdown; every item closes it, and it dismisses on outside-tap/Escape.
  let optionsOpen = $state(false);

  // Copying a note into another vault is sensitive: it writes into that vault's repo
  // (permanent in its git history). So the button opens a popover that states this plainly,
  // defaults to the most restrictive behaviour (prose only — links & files removed), and
  // every copy leaves an Undo that recedes it. `copyUndo` holds what the last copy created.
  let copyOpen = $state(false);
  let copyWithAssets = $state(false);
  // A target click arms a warning-coloured confirm when the copy is *sharper than plain prose*:
  // it carries the files (real bytes into another repo) and/or it replaces a copy already there.
  // `existing` tailors the warning; a plain, first-time prose copy skips the confirm entirely.
  let copyConfirm = $state<{ vault: string; existing: boolean } | null>(null);
  let copyUndo = $state<{ id: string; vault: string; blobs: string[]; replaced: number } | null>(
    null,
  );
  let copyUndoTimer: ReturnType<typeof setTimeout> | undefined;
  // The vaults this note can be copied to: every other one (never its own audience).
  const otherVaults = $derived(vaults.filter((v) => v && v !== note?.vault));
  let draft = $state('');
  let saved = $state(true);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Editable property fields, initialized from the note when it loads. Each maps
  // to exactly what `apply_property` (via set_property) accepts: hard is a bool,
  // tags are comma/space separated. There is no type field — notes are
  // differentiated by tags, and `asset` is set only by ingest.
  //
  // start/due are split across two inputs because the model's time is OPTIONAL:
  // a single `datetime-local` would force a time on every deadline and make
  // "sometime Tuesday" unexpressible. The date holds the stamp; the time refines
  // it. `toStamp` recombines them into the single wire value.
  let pTitle = $state('');
  let pStatus = $state('');
  let pStart = $state('');
  let pStartTime = $state('');
  let pDue = $state('');
  let pDueTime = $state('');
  let pHard = $state(false);
  let pTags = $state('');
  let propTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  // The body editor, for caret-based insertion (drag-drop + slash-menu).
  let editorEl = $state<HTMLTextAreaElement | undefined>(undefined);
  let adding = $state(false);

  // Slash-menu (Notion-style `/` → insert a note or asset reference) state.
  // `at` is where the popup sits: the caret's pixel position within the editor,
  // so the menu opens under what you are typing rather than at a fixed corner.
  type SlashState = {
    open: boolean;
    from: number;
    query: string;
    results: ObjectMeta[];
    active: number;
    // `//` opens the menu in embed mode — a tap inserts an inline embed instead of a chip link, so
    // embedding needs no Shift key (the phone has none handy). `/` stays link mode.
    embed: boolean;
    at: { top: number; left: number };
  };
  let slash = $state<SlashState>({
    open: false,
    from: -1,
    query: '',
    results: [],
    active: 0,
    embed: false,
    at: { top: 0, left: 0 },
  });
  let slashTimer: ReturnType<typeof setTimeout> | undefined;
  // The pane's root, so a global key (Ctrl+S) can tell whether *this* pane in the
  // trail is the one being typed in.
  let paneEl = $state<HTMLElement | undefined>(undefined);

  // The **version** of the body this pane last saw — a hash, not the `updated` stamp — sent
  // back with every write so the server can refuse a save based on a body that has since
  // moved. Empty until the note loads, which is also the "don't check" signal: there is
  // nothing to lose before then.
  //
  // A hash rather than a timestamp because a stamp only catches writers that bump it, and
  // hand-editing a note in Vim does not.
  let base = $state('');

  // Object URLs minted for inline assets, revoked when the note changes or the
  // panel closes so the blobs don't leak.
  let assetUrls: string[] = [];
  function revokeAssets() {
    for (const u of assetUrls) URL.revokeObjectURL(u);
    assetUrls = [];
  }
  onDestroy(revokeAssets);

  // Give render.ts a URL + sniffed MIME so it can pick the right inline element
  // (image / PDF / video / audio). null (missing blob, or the browser/test mock)
  // → the inline "not available" placeholder; media absence is a warning, never a
  // broken pane.
  //
  // Against the real server this is just a path: the element streams from
  // `/api/blob/<ref>` and range-requests what it needs, so opening a note with a
  // 300 MB video costs no memory and seeking costs one range. The object-URL path
  // below survives only for the mock backend, which has no server to stream from —
  // and it is the one that used to hold every inline asset in memory twice.
  async function resolveAsset(ref: string): Promise<ResolvedAsset | AssetFailure | null> {
    try {
      const status = await assetStatus(ref);
      if (!status.has_blob) {
        // On screen, not in the console: an Android WebView logs nothing by default and MIUI
        // suppresses the rest, so a console line is invisible on the one device that matters.
        return { reason: `no bytes in vault "${note?.vault || 'default'}" for ${ref}` };
      }
      const mime = status.mime ?? '';
      if (streamsBlobs) return { url: assetUrl(ref), mime };
      const buf = await ipcResolveAsset(ref, 'full');
      if (!buf || buf.byteLength === 0) {
        return { reason: 'the vault has this blob but it read back empty' };
      }
      const url = URL.createObjectURL(new Blob([buf], mime ? { type: mime } : undefined));
      assetUrls.push(url);
      return { url, mime };
    } catch (e) {
      return { reason: e instanceof Error ? e.message : String(e) };
    }
  }

  $effect(() => {
    note = null;
    editing = false;
    boardFull = false; // opening a different note leaves any full-screen board
    revokeAssets();
    getNote(id)
      .then((n) => {
        note = n;
        draft = n?.body ?? '';
        base = n?.version ?? '';
        if (n) {
          pTitle = n.title ?? '';
          pStatus = n.status ?? '';
          const start = parseStamp(n.start);
          const due = parseStamp(n.due);
          pStart = start?.day ?? '';
          pStartTime = start?.time ?? '';
          pDue = due?.day ?? '';
          pDueTime = due?.time ?? '';
          pHard = n.hard;
          pTags = n.tags.join(', ');
          if (startEditing) editing = true; // "New note" opens straight in the editor
        }
      })
      .catch((e) => (error = String(e)));
  });

  // Render the read view when not editing (a textarea holds the literal bytes).
  // Revoke the previous render's object URLs first — innerHTML is about to
  // replace the elements that hold them.
  $effect(() => {
    if (note && content && !editing) {
      revokeAssets();
      renderInto(content, note.body, resolveAsset, resolveNote, resolveEmbed)
        .then(enableReadWidgets)
        .catch((e) => (error = String(e)));
    }
  });

  // Make this note's own read-view widgets interactive after each render. **Embedded notes' widgets
  // stay inert** — `![](note:id)` renders another file's atom inline; it is read-only here (its owner
  // edits it in its own pane), and excluding it is what keeps the tap→source ordinals counting only
  // *our* elements.
  function enableReadWidgets() {
    if (!content) return;
    // GFM renders `- [ ]` as a *disabled* checkbox — enable ours so a tap flips the body byte.
    for (const box of content.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')) {
      if (box.closest('.note-embed')) continue;
      box.disabled = false;
      box.classList.add('task-toggle');
      box.closest('li')?.classList.toggle('task-done', box.checked); // strike done items on load
    }
    // A callout's type badge becomes a picker handle on ours (embedded callouts stay a plain label).
    for (const kind of content.querySelectorAll<HTMLElement>('.callout-kind')) {
      if (kind.closest('.note-embed')) continue;
      kind.classList.add('pickable');
    }
    // Table cells in our own tables get a text cursor — a tap edits them in place.
    for (const cell of content.querySelectorAll<HTMLElement>('td, th')) {
      if (cell.closest('.note-embed')) continue;
      cell.classList.add('cell-edit');
    }
  }

  // A `note:` chip needs the target's live title/status. `get` already returns
  // them (it over-fetches the body, which a chip ignores — not worth a second IPC
  // command until it measures). A missing note resolves to null and the chip
  // degrades to a placeholder.
  async function resolveNote(refId: string): Promise<ResolvedNote | null> {
    const n = await getNote(refId);
    return n && { id: n.id, type: n.type, title: n.title, status: n.status };
  }

  // A `![alt](note:id)` embed needs the target's body to render it inline. `get` returns it; the
  // recursion depth/cycle guard lives in render.ts. A missing target degrades to a placeholder.
  async function resolveEmbed(refId: string): Promise<ResolvedEmbed | null> {
    const n = await getNote(refId).catch(() => null);
    return n && { title: n.title, body: n.body };
  }

  // One delegated listener for every chip and checkbox in the pane — both are created by
  // render.ts, so binding per-element would mean re-binding on every render.
  function onReadClick(e: MouseEvent) {
    const el = e.target as HTMLElement | null;
    const box = el?.closest<HTMLInputElement>('input[type="checkbox"]');
    if (box && !box.closest('.note-embed')) {
      // We drive `checked` from the source we patch, not the browser's default toggle.
      e.preventDefault();
      toggleTask(box);
      return;
    }
    const kind = el?.closest<HTMLElement>('.callout-kind');
    if (kind && !kind.closest('.note-embed')) {
      e.preventDefault();
      openCalloutPicker(kind);
      return;
    }
    const chip = el?.closest<HTMLElement>('.note-chip');
    const target = chip?.dataset.noteId;
    if (target) {
      onnavigate?.(target);
      return;
    }
    const cell = el?.closest<HTMLTableCellElement>('td, th');
    if (cell && !cell.closest('.note-embed')) {
      e.preventDefault();
      openCellEditor(cell);
    }
  }

  // The byte offset of the state char inside each GFM task marker (`- [ ]` / `- [x]`), in document
  // order, skipping fenced code blocks — which `marked` also does, so the Nth position here lines up
  // exactly with the Nth rendered checkbox. A task marker is a literal token, so this ordinal is
  // exact (unlike locate.ts's word-ordinal, which invisible syntax can shift).
  function taskMarkerPositions(src: string): number[] {
    const out: number[] = [];
    let fenced = false;
    let offset = 0;
    for (const line of src.split('\n')) {
      if (/^\s*(```|~~~)/.test(line)) {
        fenced = !fenced;
      } else if (!fenced) {
        const m = /^(\s*[-*+] +)\[[ xX]\]/.exec(line);
        if (m) out.push(offset + m[1].length + 1); // the char between the brackets
      }
      offset += line.length + 1; // + the '\n'
    }
    return out;
  }

  // Toggle the tapped checkbox by flipping exactly its `[ ]`↔`[x]` byte in the source, then persist.
  // The rendered box is updated in place (no full re-render — that would re-run KaTeX/Mermaid over
  // the whole note for one tap); the read view and the source stay in agreement because a checkbox's
  // rendered state is a total function of the byte we just wrote.
  function toggleTask(box: HTMLInputElement) {
    if (!note || editing || !content) return;
    const boxes = [...content.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].filter(
      (b) => !b.closest('.note-embed'),
    );
    const idx = boxes.indexOf(box);
    const positions = taskMarkerPositions(draft);
    // If the counts ever disagree (an edge the ordinal can't resolve), bail rather than mis-toggle.
    if (idx < 0 || idx >= positions.length) return;
    const at = positions[idx];
    const checked = draft[at] !== ' ';
    draft = draft.slice(0, at) + (checked ? ' ' : 'x') + draft.slice(at + 1);
    box.checked = !checked;
    box.closest('li')?.classList.toggle('task-done', !checked);
    void saveTask();
  }

  // The callout-type picker. `at` anchors a small menu under the tapped type badge; `el`/`idx` say
  // which callout the pick rewrites. The vocabulary is the closed `CALLOUT_TYPES` — data SELECTS a
  // type from a fixed set, it never supplies one (the no-plugin-API line).
  let calloutPick = $state<{ el: HTMLElement; idx: number; top: number; left: number } | null>(null);
  const CALLOUT_OPTIONS = CALLOUT_TYPES;

  // The `[!type]` token span of each valid callout, in document order, skipping fenced code —
  // exactly the callouts `marked` renders, so the Nth here is the Nth in the read view.
  function calloutTypePositions(src: string): { at: number; len: number }[] {
    const out: { at: number; len: number }[] = [];
    let fenced = false;
    let offset = 0;
    for (const line of src.split('\n')) {
      if (/^\s*(```|~~~)/.test(line)) {
        fenced = !fenced;
      } else if (!fenced) {
        const m = /^> \[!([a-z]+)\]/.exec(line);
        if (m && (CALLOUT_TYPES as readonly string[]).includes(m[1])) {
          out.push({ at: offset + m[0].indexOf('[!') + 2, len: m[1].length });
        }
      }
      offset += line.length + 1;
    }
    return out;
  }

  function openCalloutPicker(kindEl: HTMLElement) {
    if (!note || editing || !content) return;
    const callouts = [...content.querySelectorAll<HTMLElement>('.callout')].filter(
      (c) => !c.closest('.note-embed'),
    );
    const idx = callouts.indexOf(kindEl.closest('.callout') as HTMLElement);
    if (idx < 0) return;
    const r = kindEl.getBoundingClientRect();
    calloutPick = { el: kindEl, idx, top: r.bottom + 4, left: r.left };
  }

  // Rewrite the tapped callout's `[!type]` to the chosen kind, in place (swap the class + label, no
  // whole-note re-render), and persist the one-token source change.
  function pickCallout(kind: string) {
    const pick = calloutPick;
    calloutPick = null;
    if (!pick || !note) return;
    const positions = calloutTypePositions(draft);
    if (pick.idx >= positions.length) return; // ordinal drift — bail rather than mis-edit
    const { at, len } = positions[pick.idx];
    if (draft.slice(at, at + len) === kind) return; // no change
    draft = draft.slice(0, at) + kind + draft.slice(at + len);
    const callout = pick.el.closest('.callout');
    if (callout) callout.className = `callout callout-${kind}`;
    pick.el.textContent = kind;
    pick.el.dataset.kind = kind;
    void saveTask();
  }

  // ---- Table cell editing (deliberately conservative) ----
  //
  // Tap a cell in one of this note's tables to edit it in place. It only handles a plain **bordered**
  // GFM row (`| a | b |`) and **bails** on anything it can't map exactly — ragged rows, a column past
  // the source, a non-bordered row — rather than risk mangling a table. Byte-for-byte beats coverage.
  let cellEdit = $state<
    { at: number; len: number; value: string; top: number; left: number; width: number; height: number } | null
  >(null);

  function isTableSeparator(line: string): boolean {
    return line.includes('-') && /^\s*\|?\s*:?-{1,}:?\s*(\|\s*:?-{1,}:?\s*)*\|?\s*$/.test(line);
  }

  // The lines of the Nth GFM table block in the source (skipping fenced code), each with its byte
  // offset. Index 0 is the header, 1 the separator, 2+ the body rows — matching the read DOM.
  function nthTableBlock(src: string, n: number): { text: string; offset: number }[] | null {
    const lines = src.split('\n');
    const offsets: number[] = [];
    let o = 0;
    for (const l of lines) {
      offsets.push(o);
      o += l.length + 1;
    }
    let fenced = false;
    let count = -1;
    for (let i = 0; i < lines.length; i++) {
      if (/^\s*(```|~~~)/.test(lines[i])) {
        fenced = !fenced;
        continue;
      }
      if (fenced) continue;
      if (lines[i].includes('|') && i + 1 < lines.length && isTableSeparator(lines[i + 1])) {
        let j = i + 2;
        while (j < lines.length && lines[j].includes('|') && !/^\s*(```|~~~)/.test(lines[j])) j++;
        count++;
        if (count === n) {
          const out: { text: string; offset: number }[] = [];
          for (let k = i; k < j; k++) out.push({ text: lines[k], offset: offsets[k] });
          return out;
        }
        i = j - 1;
      }
    }
    return null;
  }

  // The cells of one bordered row (`| a | b |`) as byte ranges of the content between consecutive
  // unescaped pipes. null for a non-bordered row — we don't edit those.
  function parseBorderedRow(line: string): { start: number; end: number; text: string }[] | null {
    const trimmed = line.trim();
    if (!trimmed.startsWith('|') || !trimmed.endsWith('|')) return null;
    const pipes: number[] = [];
    for (let k = 0; k < line.length; k++) if (line[k] === '|' && line[k - 1] !== '\\') pipes.push(k);
    if (pipes.length < 2) return null;
    const cells: { start: number; end: number; text: string }[] = [];
    for (let k = 0; k < pipes.length - 1; k++) {
      cells.push({ start: pipes[k] + 1, end: pipes[k + 1], text: line.slice(pipes[k] + 1, pipes[k + 1]) });
    }
    return cells;
  }

  // Map a tapped DOM cell to its exact source byte range, or null if it can't be mapped safely.
  function cellSourceSpan(cell: HTMLTableCellElement): { at: number; len: number; value: string } | null {
    if (!content) return null;
    const table = cell.closest('table');
    if (!table) return null;
    const tables = [...content.querySelectorAll('table')].filter((t) => !t.closest('.note-embed'));
    const tIdx = tables.indexOf(table);
    if (tIdx < 0) return null;
    const tr = cell.closest('tr');
    if (!tr) return null;
    const inHead = !!cell.closest('thead');
    const bodyRows = [...table.querySelectorAll('tbody tr')];
    const lineIdx = inHead ? 0 : 2 + bodyRows.indexOf(tr);
    const block = nthTableBlock(draft, tIdx);
    if (!block || lineIdx < 0 || lineIdx >= block.length) return null;
    const cells = parseBorderedRow(block[lineIdx].text);
    const col = cell.cellIndex;
    if (!cells || col < 0 || col >= cells.length) return null;
    const c = cells[col];
    return { at: block[lineIdx].offset + c.start, len: c.end - c.start, value: c.text.trim() };
  }

  function openCellEditor(cell: HTMLTableCellElement) {
    if (!note || editing) return;
    const span = cellSourceSpan(cell);
    if (!span) return; // can't map safely — leave the cell read-only
    const r = cell.getBoundingClientRect();
    cellEdit = { ...span, top: r.top, left: r.left, width: r.width, height: r.height };
  }

  // Replace just this cell's content, escaping any pipe and flattening newlines so the table can't
  // break; save re-renders the table with the new (Markdown) content — the one place a re-render is
  // the point, not a cost.
  function commitCell() {
    const c = cellEdit;
    cellEdit = null;
    if (!c || !note) return;
    const clean = c.value.replace(/[\r\n]+/g, ' ').replace(/\|/g, '\\|').trim();
    const replacement = ` ${clean} `;
    if (draft.slice(c.at, c.at + c.len) === replacement) return; // no change
    draft = draft.slice(0, c.at) + replacement + draft.slice(c.at + c.len);
    void saveTask();
  }

  function autofocus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  // A checkbox tap must not be violent on conflict. Where `save`/`onSaveRejected` reopens the editor
  // with `<<<<<<<` markers (right for a lost paragraph), a tap just re-syncs to disk and asks for a
  // re-tap — losing nothing, since the only edit was one byte we can safely redo.
  async function saveTask() {
    if (!note) return;
    try {
      base = await updateBody(note.id, draft, base);
      note = { ...note, body: draft };
      saved = true;
      onsaved?.();
    } catch (e) {
      if (String(e).includes('changed on disk')) {
        const fresh = await getNote(note.id).catch(() => null);
        if (fresh) {
          note = fresh;
          draft = fresh.body;
          base = fresh.version;
        }
        error = 'This note changed on disk — reloaded it. Tap again.';
      } else {
        error = String(e);
      }
    }
  }

  // ---- Formatting toolbar ----
  //
  // Bold, Italic, Highlight, Code, Colour, Link, and a ¶ block menu. Each wraps the **selected
  // bytes** in the right Markdown (or the closed-vocab `[…]{.token}`), byte-for-byte source.
  //
  // **Two presentations, by pointer.** Where a pointer is precise (a desktop mouse), the bar
  // *floats above the selection* — a discrete presence that appears only when you select something,
  // the way it should be. On **touch** (coarse pointer), it is a *persistent* row above the editor
  // instead, because on Android the float hides behind the system Cut/Copy menu that pops over any
  // selection. One set of actions, shown two ways.
  const coarsePointer =
    typeof window !== 'undefined' && window.matchMedia
      ? window.matchMedia('(pointer: coarse)').matches
      : false;
  let fmtBar = $state<{ top: number; left: number } | null>(null); // float position — fine pointer only
  let colorOpen = $state(false);
  let blockOpen = $state(false);

  // Position (or hide) the floating bar for the current selection. A no-op on touch, where the bar
  // is always shown.
  function onEditorSelect() {
    if (coarsePointer) return;
    const el = editorEl;
    if (!el) return;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    if (s === e) {
      fmtBar = null;
      colorOpen = false;
      blockOpen = false;
      return;
    }
    const { top, left } = caretXY(el, s);
    const BAR_H = 40;
    fmtBar = { top: top - BAR_H < 0 ? top + 22 : top - BAR_H, left: clamp(left, 220, el.clientWidth) };
  }

  // Wrap (or, if already wrapped, unwrap — a real toggle) the selection with `before`/`after`.
  async function wrapSel(before: string, after: string, placeholder = '') {
    const el = editorEl;
    if (!el) return;
    colorOpen = false;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const inner = draft.slice(s, e);
    const wrapped = draft.slice(s - before.length, s) === before && draft.slice(e, e + after.length) === after;
    if (wrapped) {
      draft = draft.slice(0, s - before.length) + inner + draft.slice(e + after.length);
      onInput();
      await tick();
      el.focus();
      el.selectionStart = s - before.length;
      el.selectionEnd = e - before.length;
    } else {
      const text = inner || placeholder;
      draft = draft.slice(0, s) + before + text + after + draft.slice(e);
      onInput();
      await tick();
      el.focus();
      el.selectionStart = s + before.length;
      el.selectionEnd = s + before.length + text.length;
    }
    onEditorSelect();
  }

  // A link keeps the selected text as the label and drops the caret in an empty `()` to type the URL.
  async function insertLink() {
    const el = editorEl;
    if (!el) return;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const text = draft.slice(s, e) || 'text';
    draft = draft.slice(0, s) + `[${text}]()` + draft.slice(e);
    onInput();
    await tick();
    el.focus();
    const caret = s + text.length + 3; // after "[text]("
    el.selectionStart = el.selectionEnd = caret;
    fmtBar = null;
  }

  // Block formats work on whole lines: expand the selection to the lines it touches, transform each,
  // and reselect the result. Byte-for-byte source, same as the inline wraps.
  async function transformLines(fn: (lines: string[]) => string[]) {
    const el = editorEl;
    if (!el) return;
    colorOpen = false;
    blockOpen = false;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const from = draft.lastIndexOf('\n', s - 1) + 1;
    const nl = draft.indexOf('\n', e);
    const to = nl === -1 ? draft.length : nl;
    const out = fn(draft.slice(from, to).split('\n')).join('\n');
    draft = draft.slice(0, from) + out + draft.slice(to);
    onInput();
    await tick();
    el.focus();
    el.selectionStart = from;
    el.selectionEnd = from + out.length;
    onEditorSelect();
  }

  // Set the heading level, replacing any marker already there (so H2 over an H1 line just re-levels).
  const heading = (n: number) =>
    transformLines((lines) => lines.map((l) => '#'.repeat(n) + ' ' + l.replace(/^#{1,6}\s+/, '')));
  // Bullet list — a toggle: if every line is already a bullet, strip it.
  const bullets = () =>
    transformLines((lines) =>
      lines.every((l) => /^\s*[-*+]\s+/.test(l))
        ? lines.map((l) => l.replace(/^(\s*)[-*+]\s+/, '$1'))
        : lines.map((l) => '- ' + l.replace(/^\s*[-*+]\s+/, '')),
    );
  const numbered = () =>
    transformLines((lines) => lines.map((l, i) => `${i + 1}. ` + l.replace(/^\s*\d+\.\s+/, '')));
  const quoteSel = () => transformLines((lines) => lines.map((l) => `> ${l.replace(/^>\s?/, '')}`));
  // Callout: a blockquote whose first line carries `[!note]` — the read-view badge then re-types it.
  const calloutBlock = () =>
    transformLines((lines) =>
      lines.map((l, i) =>
        i === 0
          ? `> [!note] ${l.replace(/^>\s?(\[!\w+\]\s?)?/, '')}`
          : `> ${l.replace(/^>\s?/, '')}`,
      ),
    );

  // Debounced save: typing stops -> 500 ms -> atomic write via update_body.
  // Also re-evaluate the slash-menu trigger against the new caret.
  function onInput() {
    saved = false;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 500);
    detectSlash();
    onEditorSelect(); // typing replaces the selection → refresh/hide the float
  }

  async function save() {
    if (!note) return;
    try {
      base = await updateBody(note.id, draft, base);
      note = { ...note, body: draft };
      saved = true;
      onsaved?.();
    } catch (e) {
      await onSaveRejected(e);
    }
  }

  // The pane has been open across someone else's pull, and the note it is holding is no
  // longer the note on disk. The server refuses that write rather than letting a debounced
  // auto-save overwrite a merged paragraph — so what is left is to say so and show what
  // actually landed.
  //
  // **The draft is not thrown away.** It goes back in the editor beside their text, because
  // the one thing worse than a conflict is a conflict that ate what you were writing. This
  // is the same stance as the `.md` driver's: put both versions where a human can see them
  // and let them decide.
  async function onSaveRejected(e: unknown) {
    if (!note || !String(e).includes('changed on disk')) {
      error = String(e);
      return;
    }
    const mine = draft;
    const fresh = await getNote(note.id).catch(() => null);
    if (!fresh) {
      error = String(e);
      return;
    }
    note = fresh;
    base = fresh.updated;
    draft = `${fresh.body}\n\n<<<<<<< your unsaved edit\n${mine}\n>>>>>>>\n`;
    saved = false;
    error =
      'Someone else changed this note while you had it open. Their version is above; ' +
      'your unsaved edit is marked below it — merge the two and save.';
  }

  // A board note carries `view: board`; its body is an Excalidraw scene (JSON),
  // edited on a canvas instead of as text. Detection is a plain property check —
  // no new Kind (the model resists that).
  let isBoard = $derived(note?.props?.view === 'board');

  // A first-class discussion — a note that is the root of its own thread (`thread_of` points at
  // itself, the self-anchor `create_discussion` writes). Its content is the conversation, not a
  // document, so the pane suppresses the read/edit body and shows the thread as the body.
  let isDiscussion = $derived(!!note && note.props?.thread_of === `note:${note.id}`);

  // Copy-to only makes sense when there is another vault to copy into — and not for a discussion,
  // whose "body" is a thread, so copying its (empty) prose is meaningless. Derived once so the
  // overflow item's guard and the copy popover's guard cannot disagree.
  let canCopy = $derived(otherVaults.length > 0 && !isDiscussion);

  // ── Discussion ────────────────────────────────────────────────────────────────────────
  // Collapsed by default and fetched on open. That is not laziness for its own sake: a
  // thread read is one full-corpus query on the server, so paying it on every note open
  // would put it on the hot path for notes nobody has discussed.
  let discOpen = $state(false);
  let discCount = $state(0);
  let discMessages = $state<ThreadMessage[]>([]);

  // "Linked from": notes whose body references this one. Fetched when the note changes (a store
  // scan on the server, no reverse index); shown only when there is at least one.
  let backRefs = $state<ObjectMeta[]>([]);
  $effect(() => {
    const id = note?.id;
    backRefs = [];
    if (id) void ipcBacklinks(id).then((r) => (backRefs = r)).catch(() => {});
  });
  let replyDraft = $state('');
  let replyTo = $state('');
  let discBusy = $state(false);
  let discError = $state<string | null>(null);

  async function loadThread() {
    if (!note) return;
    try {
      const t = await ipcThread(note.id);
      discMessages = t.messages;
      discCount = t.count;
      discError = null;
    } catch (e) {
      discError = String(e);
    }
  }

  async function toggleDiscussion() {
    discOpen = !discOpen;
    if (discOpen) {
      replyTo = note?.id ?? '';
      await loadThread();
    }
  }

  // A discussion opens straight onto its conversation: the thread is shown and fetched as soon as
  // the note loads, never collapsed. A plain note starts collapsed (and this also clears a prior
  // discussion's open state when the pane switches notes). Runs on note-change, not on toggle —
  // it reads `note?.id`/`isDiscussion`, so a manual toggle below does not re-trigger it.
  $effect(() => {
    const id = note?.id;
    if (isDiscussion && id) {
      discOpen = true;
      replyTo = id;
      void loadThread();
    } else if (!isDiscussion) {
      discOpen = false;
    }
  });

  // Rename a discussion. Its title is the at-a-glance label in the Discussions view, and a
  // discussion has no edit mode (its body is the thread), so it is set here directly.
  async function renameDiscussion(title: string) {
    if (!note) return;
    try {
      await setProperty(note.id, 'title', title);
      note = { ...note, title: title || null };
      onsaved?.();
    } catch (e) {
      discError = String(e);
    }
  }

  function startReply(id: string) {
    replyTo = id;
    discError = null;
  }

  async function sendReply() {
    const body = replyDraft.trim();
    if (!note || !body || discBusy) return;
    discBusy = true;
    discError = null;
    try {
      // The target is the message being answered, or the note itself. Replying to a message
      // re-roots server-side, so this cannot create a thread nothing can reach.
      await ipcReply(replyTo || note.id, body);
      replyDraft = '';
      replyTo = note.id;
      await loadThread();
      // A message is a file write like any other, so it rides the same signal the editor
      // uses — which is what schedules the debounced commit. No second path.
      onsaved?.();
    } catch (e) {
      discError = String(e);
    } finally {
      discBusy = false;
    }
  }

  function onReplyKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      void sendReply();
    }
  }
  let boardTheme: 'dark' | 'light' = $derived(
    document.documentElement.dataset.theme === 'light' ? 'light' : 'dark',
  );

  // The canvas persists the same way the textarea does: write the body bytes.
  async function saveBoard(json: string) {
    if (!note) return;
    try {
      base = await updateBody(note.id, json, base);
      note = { ...note, body: json };
      onsaved?.();
    } catch (e) {
      // A canvas cannot show conflict markers, so a board says what happened and reloads
      // rather than pretending to merge. `scene.rs` already merged the two scenes on the
      // way in — what is being refused here is only this stale re-serialization of it.
      if (String(e).includes('changed on disk')) {
        const fresh = await getNote(note.id).catch(() => null);
        if (fresh) {
          note = fresh;
          base = fresh.updated;
        }
        error =
          'This board changed while you had it open — the merged version has been ' +
          'reloaded. Any strokes from the last few seconds may need redrawing.';
      } else {
        error = String(e);
      }
    }
  }

  // Write one property via the existing set_property command, then reflect it in
  // the local note so the header/pills update without a refetch.
  async function setProp(key: string, value: string) {
    if (!note) return;
    try {
      await setProperty(note.id, key, value);
      applyLocal(key, value);
      onsaved?.();
      error = null;
    } catch (e) {
      error = String(e);
    }
  }
  // Text fields debounce so a burst of typing isn't one write per keystroke.
  function setPropDebounced(key: string, value: string) {
    clearTimeout(propTimers[key]);
    propTimers[key] = setTimeout(() => setProp(key, value), 400);
  }

  // Write a start/due from its date+time pair. Clearing the DATE clears the whole
  // property (and drops the now-orphaned time), because a time with no day isn't
  // a point on any calendar.
  function setStamp(key: 'start' | 'due') {
    const day = key === 'start' ? pStart : pDue;
    const time = key === 'start' ? pStartTime : pDueTime;
    if (!day) {
      if (key === 'start') pStartTime = '';
      else pDueTime = '';
    }
    void setProp(key, toStamp(day, time));
  }
  function applyLocal(key: string, value: string) {
    if (!note) return;
    if (key === 'title') note = { ...note, title: value || null };
    else if (key === 'status') note = { ...note, status: value || null };
    else if (key === 'due') note = { ...note, due: value || null };
    else if (key === 'start') note = { ...note, start: value || null };
    else if (key === 'hard') note = { ...note, hard: value === 'true' };
    else if (key === 'tags')
      note = { ...note, tags: value ? value.split(/[,\s]+/).filter(Boolean) : [] };
  }

  async function toggleEdit() {
    // A board's body is the canvas scene (autosaved by Whiteboard); the textarea
    // `draft` is stale for it, so flushing it here would clobber the drawing.
    if (editing && !isBoard) await save(); // leaving edit mode flushes the textarea
    editing = !editing;
  }

  // Leaving the editor should feel like putting the pen down, not hunting for a "Done" button:
  // clicking the header chrome — the title and the note's identity line — flushes the draft and
  // drops back to the read view, the same as Ctrl+S or Escape. An intuitive trigger beats an
  // explicit command. Only the header's own controls keep their meaning (the ＋ options and its
  // window, ＋ Media, full-screen, close); everything else in the header is "done".
  function onHeaderClick(e: MouseEvent) {
    if (!editing) return;
    if ((e.target as HTMLElement | null)?.closest('button, input, a, select, .capture, .options-window')) {
      return;
    }
    void toggleEdit();
  }

  // Double-click the read view to edit it — there is no Edit button any more.
  async function startEdit(e: MouseEvent) {
    // Skip the targets that already mean something: a reference chip navigates, a
    // link follows, media has its own controls, and double-clicking to select a
    // word inside them should not throw you into the editor.
    if ((e.target as HTMLElement | null)?.closest('.note-chip, a, button, input, video, audio, iframe')) {
      return;
    }
    await openEditor(clickedOffset());
  }

  /**
   * Which source offset did the double-click land on? The gesture has already
   * selected the word under the pointer, so we count which occurrence of that
   * word it is in the rendered text and find the same one in the source — the
   * HTML carries no source positions to consult (see locate.ts).
   *
   * Must run *before* `editing` flips: the swap destroys the rendered DOM this
   * reads. Falls back to the end of the note, which is what a double-click in
   * the whitespace under a short note means anyway.
   */
  function clickedOffset(): number {
    const sel = window.getSelection();
    const word = sel?.toString().trim() ?? '';
    if (!word || !sel?.anchorNode || !content) return draft.length;
    const before = document.createRange();
    before.selectNodeContents(content);
    try {
      before.setEnd(sel.anchorNode, sel.anchorOffset);
    } catch {
      return draft.length; // selection escaped the read view — no ordinal to count
    }
    const hit = nthIndexOf(draft, word, countOf(before.toString(), word));
    return hit >= 0 ? hit : draft.length;
  }

  async function openEditor(at?: number) {
    editing = true;
    await tick();
    editorEl?.focus();
    if (at === undefined || !editorEl) return;
    editorEl.selectionStart = editorEl.selectionEnd = at;
    // focus() alone scrolls to the top, not to the caret — centre it by hand.
    const { top } = caretXY(editorEl, at);
    editorEl.scrollTop += top - editorEl.clientHeight / 2;
  }

  /** Does the keyboard focus live in *this* pane? */
  const focused = () => !!paneEl?.contains(document.activeElement);

  // Every pane in the trail mounts this window listener, so a key press is heard
  // by all of them. When focus is inside *some* pane, only that pane may act —
  // otherwise Escape in pane 2's editor would also fire pane 1's close. With
  // focus outside every pane, they all act, which is the old behavior.
  function ownsKeys(): boolean {
    const active = document.activeElement as HTMLElement | null;
    return focused() || !active?.closest?.('.panel');
  }

  function onPaneKey(e: KeyboardEvent) {
    if (!ownsKeys()) return;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 's') {
      // The app owns Ctrl+S while a note is open — suppress the browser's own
      // "save page" dialog even in the read view, where there is nothing to flush.
      e.preventDefault();
      if (editing) void toggleEdit(); // flush, then back to the read view
      return;
    }
    if (e.key !== 'Escape') return;
    // While editing, Escape leaves the editor rather than closing the pane —
    // otherwise Ctrl+S is the only way out, which is a trap if you don't know it.
    // The 500 ms debounced autosave means nothing is lost either way. The slash
    // menu consumes Escape first (onEditorKeydown), so this never fights it.
    if (editing && !slash.open) {
      e.preventDefault();
      void toggleEdit();
      return;
    }
    onclose();
  }

  // Delete, confirmed. Removes the note file + index rows, then schedules the
  // git auto-commit of the removal and closes the panel (which refreshes the view).
  async function confirmDelete() {
    if (!note) return;
    try {
      await deleteNote(note.id);
      onsaved?.();
      onclose();
    } catch (e) {
      error = String(e);
      confirmingDelete = false;
    }
  }

  function flashNotice(msg: string) {
    notice = msg;
    setTimeout(() => (notice = null), 3000);
  }

  // A target was picked. Ask the backend whether that vault already holds a copy, then decide:
  // a plain first-time prose copy runs straight away (it leaks nothing and replaces nothing);
  // carrying the files OR replacing an existing copy first arms the warning-coloured confirm,
  // still fully reversible before it runs.
  async function requestCopy(target: string) {
    if (!note) return;
    let existing = false;
    try {
      existing = await copyStatus(note.id, target);
    } catch {
      /* if the check fails, fall through — the copy itself still reports what it replaced */
    }
    if (copyWithAssets || existing) copyConfirm = { vault: target, existing };
    else void doCopy(target);
  }

  // Copy this note into `target`. Restrictive by default (only the prose travels);
  // `copyWithAssets` opts in to carrying the files. Stash what it created so Undo can
  // recede it, and auto-dismiss the Undo strip after a while so it doesn't linger.
  async function doCopy(target: string) {
    if (!note) return;
    copyOpen = false;
    copyConfirm = null;
    error = null;
    try {
      const r = await copyNote(note.id, target, copyWithAssets);
      copyUndo = { id: r.meta.id, vault: target, blobs: r.new_blobs, replaced: r.replaced };
      clearTimeout(copyUndoTimer);
      copyUndoTimer = setTimeout(() => (copyUndo = null), 12000);
      onsaved?.();
    } catch (e) {
      error = String(e);
    }
  }
  async function undoCopy() {
    const u = copyUndo;
    if (!u) return;
    copyUndo = null;
    clearTimeout(copyUndoTimer);
    try {
      await uncopyNote(u.id, u.vault, u.blobs);
      onsaved?.();
    } catch (e) {
      error = String(e);
    }
  }
  onDestroy(() => clearTimeout(copyUndoTimer));

  // ---- Editor: caret insert, drag-drop ingest, and the slash-menu ----
  async function insertAtCaret(text: string) {
    const el = editorEl;
    if (!el) return;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    draft = draft.slice(0, start) + text + draft.slice(end);
    onInput(); // schedule the debounced save
    await tick(); // let bind:value flush before moving the caret
    const pos = start + text.length;
    el.selectionStart = el.selectionEnd = pos;
    el.focus();
  }

  /** Escape the label half of a Markdown link — the URL half is always a pure
   *  hash or ULID, so only this can break the syntax. */
  function escapeLabel(text: string): string {
    return text.replace(/\\/g, '\\\\').replace(/[[\]]/g, '\\$&');
  }

  // The Markdown reference for an ingested asset.
  function assetRef(meta: ObjectMeta): string {
    const hash = meta.assets[0]?.replace(/^sha256:/, '') ?? '';
    return `![${escapeLabel(meta.title ?? 'asset')}](asset:sha256-${hash})`;
  }

  // The Markdown reference for another note. Deliberately the same shape as
  // assetRef: an ordinary Markdown link on a custom scheme, so marked needs no
  // help and the raw file still reads as prose. The ULID (not the title) is the
  // target, so retitling the other note never breaks this link.
  function noteRef(meta: ObjectMeta): string {
    return `[${escapeLabel(meta.title ?? 'note')}](note:${meta.id})`;
  }

  /** Both kinds insert through the `/` menu; the type picks the syntax. */
  function refFor(meta: ObjectMeta): string {
    return meta.type === 'asset' ? assetRef(meta) : noteRef(meta);
  }

  // The embed form of a note reference — image syntax on the `note:` scheme, so the target renders
  // inline instead of as a chip (`resolveEmbeds` in render.ts). The title rides along as the alt so
  // a missing target still degrades to a labelled placeholder. Assets are already `![…](asset:…)`,
  // so an embed request on one is just its ordinary ref.
  function embedFor(meta: ObjectMeta): string {
    return meta.type === 'asset' ? assetRef(meta) : `![${escapeLabel(meta.title ?? 'note')}](note:${meta.id})`;
  }

  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes('Files')) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    }
  }
  // ── Capture from the device ────────────────────────────────────────────────────────────
  //
  // **Deliberately a plain `<input type="file">`, and no native code at all.** Android turns
  // `capture="environment"` into the system camera Intent and a bare `accept` into the system
  // picker, so the platform's own capture UI does the work and hands back a `File` — which is
  // exactly what `ingestFile` already takes. A Tauri plugin with a Kotlin `ActivityResultLauncher`
  // would reimplement, worse, what the WebView already brokers.
  //
  // **Verified, not assumed** — wry 0.55.1 `src/android/kotlin/RustWebChromeClient.kt:272`
  // implements `onShowFileChooser`, including multi-select and the capture Intent.
  //
  // And the permission works out in our favour. Its `isMediaCaptureSupported` reads:
  //
  //     hasPermissions(activity, CAMERA) || !hasDefinedPermission(activity, CAMERA)
  //
  // — satisfied when CAMERA is granted *or is not declared at all*. This app declares only
  // INTERNET, so capture goes straight to the system camera Intent, which owns its own
  // permissions. **Declaring CAMERA would make this worse**, not better: it would add a prompt
  // for something the camera app already asks about. So the manifest stays as it is.
  //
  // Screenshots are absent on purpose: Android's own screenshot is a hardware gesture that lands
  // in the gallery, and "Photo library" then inserts it. Building a second path for something the
  // OS does better is the kind of surface this project declines.
  const CAPTURE = [
    { label: 'Take a photo', accept: 'image/*', capture: 'environment' },
    { label: 'Record a video', accept: 'video/*', capture: 'environment' },
    // **Not "Record audio".** wry only treats `capture` as capture for `image/*` and `video/*`
    // (`RustWebChromeClient.kt:279-281`); anything else falls through to `showFilePicker`, so an
    // audio input opens a file browser no matter what `capture` says. Labelling it "Record"
    // promised a recorder and delivered a folder. In-app recording needs `getUserMedia` +
    // `MediaRecorder`, the RECORD_AUDIO permission, and wry's permission plumbing — a real piece
    // of work, and not one to imply is already done.
    { label: 'Choose an audio file', accept: 'audio/*', capture: '' },
    { label: 'From the library', accept: 'image/*,video/*', capture: '' },
    { label: 'Any file', accept: '', capture: '' },
  ] as const;

  let captureEl = $state<HTMLInputElement | undefined>(undefined);
  let captureOpen = $state(false);

  /** Point the one hidden input at a source and open it. One element, reconfigured, so the
   *  browser never holds five pickers' worth of state. */
  function capture(kind: (typeof CAPTURE)[number]) {
    captureOpen = false;
    if (!captureEl) return;
    captureEl.accept = kind.accept;
    if (kind.capture) captureEl.setAttribute('capture', kind.capture);
    else captureEl.removeAttribute('capture');
    captureEl.value = ''; // so picking the same file twice still fires `change`
    captureEl.click();
  }

  /** The captured file, through exactly the path a dropped one takes. */
  async function onCaptured(e: Event) {
    const files = Array.from((e.target as HTMLInputElement).files ?? []);
    if (files.length) await ingestAll(files);
  }

  async function onDrop(e: DragEvent) {
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (!files.length) return;
    e.preventDefault();
    await ingestAll(files);
  }

  /** Ingest files and insert a reference for each at the caret — the one implementation shared
   *  by drag-drop and device capture, so a photo taken on a phone and a file dropped on a
   *  desktop cannot end up behaving differently. */
  async function ingestAll(files: File[]) {
    adding = true;
    error = null;
    const copied: string[] = [];
    try {
      for (const f of files) {
        // Into the vault of the note you dropped it on: an asset belongs to the same
        // audience as the note that references it. Dropping a PDF on a lab note and
        // having it land in your personal vault would put the bytes on the wrong side
        // of a boundary — and the note would still render it, so nothing would look
        // wrong.
        const meta = await ingestFile(f, note?.vault ?? '');
        await insertAtCaret(assetRef(meta) + '\n');
        copied.push(meta.title ?? f.name);
      }
      onsaved?.();
      // The bytes are copied into the vault's blob store; the original file on
      // disk is untouched. Confirm that plainly, as the user asked.
      flashNotice(
        copied.length === 1
          ? `Copied ${copied[0]} into the vault`
          : `Copied ${copied.length} files into the vault`,
      );
    } catch (err) {
      error = String(err);
    } finally {
      adding = false;
    }
  }

  function onEditorKeydown(e: KeyboardEvent) {
    if (!slash.open || !slash.results.length) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      slash = { ...slash, active: (slash.active + 1) % slash.results.length };
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      slash = { ...slash, active: (slash.active - 1 + slash.results.length) % slash.results.length };
    } else if (e.key === 'Enter') {
      // Enter inserts a chip link; Shift+Enter inserts an inline embed. preventDefault stops the
      // textarea's own newline — but only while the menu is open (we returned early otherwise).
      e.preventDefault();
      chooseSlash(slash.results[slash.active], e.shiftKey);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeSlash();
    }
  }

  // Detect a `/query` token at the caret (line-start or after whitespace); a
  // space dismisses. Debounced FTS on the query.
  function detectSlash() {
    const el = editorEl;
    if (!el) return closeSlash();
    const caret = el.selectionStart;
    let i = caret - 1;
    while (i >= 0 && !/\s/.test(draft[i]) && draft[i] !== '/') i--;
    if (i < 0 || draft[i] !== '/') return closeSlash();
    // A second `/` immediately before this one = embed mode (tap-friendly, no Shift). The token
    // starts at the first slash so both are replaced on insert; the query is what follows both.
    const embed = i - 1 >= 0 && draft[i - 1] === '/';
    const from = embed ? i - 1 : i;
    // The token must begin the line or follow whitespace — checking the char before the token, so
    // `https://` (the `//` follows a `:`) never triggers it.
    if (from !== 0 && !/\s/.test(draft[from - 1])) return closeSlash();
    const query = draft.slice(i + 1, caret);
    if (/\s/.test(query)) return closeSlash();
    slash = { ...slash, open: true, from, query, active: 0, embed, at: slashAnchor(el, from) };
    clearTimeout(slashTimer);
    slashTimer = setTimeout(runSlashSearch, 150);
  }

  // Put the menu just under the `/` you typed. Measured against the textarea and
  // clamped to it, so a `/` near the right or bottom edge doesn't push the popup
  // out of the pane. Where there is no layout engine (jsdom) caretXY reports
  // zeros and this degrades to the editor's top-left — never a crash.
  function slashAnchor(el: HTMLTextAreaElement, from: number): { top: number; left: number } {
    const { top, left, lineHeight } = caretXY(el, from);
    const MENU_W = 256; // 16rem, the popup's min-width
    const MENU_H = 224; // 14rem, its max-height
    const below = top + lineHeight;
    // No room underneath? Flip above the caret line rather than clamp onto it.
    const flip = below + MENU_H > el.clientHeight && top - MENU_H >= 0;
    return {
      top: flip ? top - MENU_H : clamp(below, MENU_H, el.clientHeight),
      left: clamp(left, MENU_W, el.clientWidth),
    };
  }

  async function runSlashSearch() {
    const q = slash.query.trim();
    try {
      // A bare `/` suggests your most recent notes — the overwhelmingly common
      // link target, and it makes the menu useful before you know what to type.
      // Typing switches to FTS across the whole vault, which (unlike `recent`)
      // still includes assets, so `/` remains the way to insert one.
      // The type decides the syntax at insert. Drop this note itself: a
      // self-reference is never what the `/` menu is for.
      const all = q ? await search(q) : await recent();
      slash = { ...slash, results: all.filter((n) => n.id !== id).slice(0, 8), active: 0 };
    } catch {
      slash = { ...slash, results: [] };
    }
  }
  function closeSlash() {
    if (slash.open) slash = { ...slash, open: false, results: [] };
  }
  // Inserts an inline embed when the menu is in embed mode (`//`) OR when forced (Shift+Enter /
  // Shift-click); otherwise a chip link. The `//` path is the one that works with a tap alone.
  async function chooseSlash(meta: ObjectMeta, forceEmbed = false) {
    const el = editorEl;
    if (!el) return;
    const caret = el.selectionStart;
    const ref = forceEmbed || slash.embed ? embedFor(meta) : refFor(meta);
    draft = draft.slice(0, slash.from) + ref + draft.slice(caret);
    closeSlash();
    onInput();
    await tick();
    const pos = slash.from + ref.length;
    el.selectionStart = el.selectionEnd = pos;
    el.focus();
  }
</script>

<svelte:window onkeydown={onPaneKey} onpopstate={onPopState} />

<article class="panel" class:wide class:solo class:board-full={boardFull} bind:this={paneEl}>
    {#if boardFull}
      <!-- The only chrome in full-screen: a small floating exit. Back button does the same. -->
      <button class="board-exit" onclick={() => history.back()} aria-label="exit full screen" title="Exit full screen (or press Back)">✕</button>
    {/if}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <header
      class:editing={!!(note && editing)}
      onclick={onHeaderClick}
      title={note && editing ? 'Click here (or press Ctrl+S / Esc) to finish editing' : undefined}>
      <!-- **The title gets its own line.** It used to share one flex row with the vault badge,
           the last editor, the status chip and every action button, so a title of any real
           length was squeezed into whatever those left over — unreadable on a narrow pane and
           worse on a phone. The title is what identifies the note; the controls act on it.
           Two rows, in that order. -->
      <div class="title-row">
        {#if note && note.type === 'asset'}<span class="type" data-type={note.type}>{note.type}</span>{/if}
        <h2>{note?.title ?? 'note'}</h2>
      </div>
      <div class="control-row">
      {#if note?.vault}<VaultBadge vault={note.vault} />{/if}
      {#if note}<EditedBy edit={lastEditFor(note.id)} />{/if}
      {#if note}
        <!-- **One button, one window.** Only identity — vault (audience) + who last edited — stays
             on the row; Edit, Copy, Delete (and the properties, via Edit) live behind this single
             `＋`, which opens an options *window* (a card, not a dropdown). Keeps a phone header
             legible; refinement is one tap away when wanted. The window is nested in this
             `position:relative` wrapper — the same shape as `＋ Media` — so it opens *directly
             under the button* at any scroll offset, never adrift at the panel's edge. -->
        <div class="options" use:clickOutside={() => (optionsOpen = false)}>
          <button
            class="edit options-btn"
            onclick={() => (optionsOpen = !optionsOpen)}
            aria-haspopup="dialog"
            aria-expanded={optionsOpen}
            aria-label="note options"
            title="Options">＋</button>
          {#if optionsOpen}
            <!-- Dismissed by tapping outside (the wrapper's clickOutside), Escape, or its ✕ —
                 identically on a phone and a laptop. Each action closes it, so it and a popover are
                 never both open. Edit reveals the property form (the fine refinement). -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="options-window"
              role="dialog"
              tabindex="-1"
              aria-label="note options"
              onkeydown={(e) => e.key === 'Escape' && (optionsOpen = false)}>
              <div class="options-head">
                <span>Options</span>
                <button class="opt-close" onclick={() => (optionsOpen = false)} aria-label="close options">✕</button>
              </div>
              {#if !isDiscussion}
                <button class="opt" onclick={() => { optionsOpen = false; void toggleEdit(); }}>
                  {#if isBoard}{editing ? 'Done' : 'Details'}{:else}{editing ? 'Done' : 'Edit'}{/if}
                </button>
              {/if}
              {#if note.type === 'asset' && note.assets.length}
                <button class="opt" onclick={() => { optionsOpen = false; openExternal(note!.assets[0]).catch((e) => (error = String(e))); }}>Open externally</button>
              {/if}
              {#if canCopy}
                <button class="opt" onclick={() => { optionsOpen = false; copyOpen = true; }}>Copy to…</button>
              {/if}
              <button class="opt danger" onclick={() => { optionsOpen = false; confirmingDelete = true; }}>Delete</button>
            </div>
          {/if}
        </div>
      {/if}
      {#if note && editing}
        <!-- Only while editing: capture exists to put something *into* the text you are
             writing, and the caret it inserts at only means something in the editor. -->
        <div class="capture" use:clickOutside={() => (captureOpen = false)}>
          <button
            class="edit"
            onclick={() => (captureOpen = !captureOpen)}
            aria-expanded={captureOpen}
            aria-label="add media">＋ Media</button>
          {#if captureOpen}
            <ul class="capture-menu">
              {#each CAPTURE as kind (kind.label)}
                <li>
                  <button onclick={() => capture(kind)}>{kind.label}</button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
        <!-- One hidden input, reconfigured per source. `multiple` because the library picker is
             the natural place to add several at once, and `ingestAll` already loops. -->
        <input
          class="capture-input"
          type="file"
          multiple
          bind:this={captureEl}
          onchange={onCaptured} />
      {/if}
      <button
        class="icon-toggle"
        onclick={isBoard ? enterBoardFull : ontogglewide}
        aria-pressed={isBoard ? boardFull : wide}
        aria-label="full screen"
        title={isBoard ? 'Full screen board (Back to exit)' : wide ? 'Exit full screen' : 'Full screen'}>
        {isBoard ? '⛶' : wide ? '⤡' : '⤢'}
      </button>
      <button class="close" onclick={onclose} aria-label="close">✕</button>
          </div>
    </header>
    {#if confirmingDelete}
      <div class="confirm" role="alertdialog" aria-label="confirm delete">
        <span>Delete this note permanently? This can't be undone.</span>
        <div class="confirm-actions">
          <button class="edit" onclick={() => (confirmingDelete = false)}>Cancel</button>
          <button class="edit danger solid" onclick={confirmDelete}>Delete</button>
        </div>
      </div>
    {/if}
    {#if copyOpen && otherVaults.length}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="copy-pop"
        role="dialog"
        tabindex="-1"
        aria-label="copy to another vault"
        use:clickOutside={() => (copyOpen = false)}
        onkeydown={(e) => e.key === 'Escape' && (copyOpen = false)}>
        <p class="copy-warn">
          Copying writes a <strong>new note</strong> into another vault's repository —
          <strong>permanent in that vault's git history</strong>. By default only the text is
          copied; links and attached files are removed. Pick a vault:
        </p>
        <label class="copy-opt">
          <input type="checkbox" bind:checked={copyWithAssets} aria-label="also copy the files" />
          Also copy the files into that vault
        </label>
        {#if copyConfirm}
          {@const cc = copyConfirm}
          <!-- The sharper confirm: replacing an existing copy and/or carrying the files, both
               permanent in that vault's history. Warning-coloured, and still reversible —
               Cancel backs out before anything runs. -->
          <div class="copy-danger" role="alertdialog" aria-label="confirm copy">
            <span>
              <strong>{cc.vault}</strong>
              {#if cc.existing}
                already has a copy of this note — copying again replaces it{#if copyWithAssets}, and
                  writes its files there{/if}. Written into that vault's repository — permanent in
                its git history.
              {:else}
                — copy the note and its files here. Written into that vault's repository —
                permanent in its git history.
              {/if}
            </span>
            <div class="confirm-actions">
              <button class="edit" onclick={() => (copyConfirm = null)}>Cancel</button>
              <button class="edit danger solid" onclick={() => doCopy(cc.vault)}>
                {cc.existing ? 'Replace copy' : 'Copy with files'}
              </button>
            </div>
          </div>
        {:else}
          <div class="copy-targets">
            {#each otherVaults as v (v)}
              <button class="edit" onclick={() => requestCopy(v)} title={`Copy into ${v}`}>{v}</button>
            {/each}
            <button class="edit" onclick={() => (copyOpen = false)}>Cancel</button>
          </div>
        {/if}
      </div>
    {/if}
    {#if copyUndo}
      <div class="copy-undo" role="status">
        <span>
          {copyUndo.replaced > 0
            ? `Replaced the copy in ${copyUndo.vault}.`
            : `Copied to ${copyUndo.vault}.`}
        </span>
        <button class="edit" onclick={undoCopy}>Undo</button>
      </div>
    {/if}
    {#if error}
      <p class="err">{error}</p>
    {/if}
    {#if notice}
      <p class="note-notice">{notice}</p>
    {/if}
    {#if note}
      {#if editing}
        <div class="props">
          <label class="field">
            <span>Status</span>
            <input
              aria-label="status"
              list="np-statuses"
              bind:value={pStatus}
              oninput={() => setPropDebounced('status', pStatus)}
              placeholder="e.g. todo, doing, done"
              spellcheck="false"
            />
            <datalist id="np-statuses">
              {#each statuses as st (st)}<option value={st}></option>{/each}
            </datalist>
          </label>
          <label class="field">
            <span>Start</span>
            <span class="when">
              <input
                aria-label="start"
                type="date"
                bind:value={pStart}
                onchange={() => setStamp('start')}
              />
              <input
                aria-label="start time"
                type="time"
                bind:value={pStartTime}
                onchange={() => setStamp('start')}
                disabled={!pStart}
                title={pStart ? 'Optional — leave empty for an all-day item' : 'Set a start date first'}
              />
            </span>
          </label>
          <label class="field">
            <span>Due</span>
            <span class="when">
              <input aria-label="due" type="date" bind:value={pDue} onchange={() => setStamp('due')} />
              <input
                aria-label="due time"
                type="time"
                bind:value={pDueTime}
                onchange={() => setStamp('due')}
                disabled={!pDue}
                title={pDue ? 'Optional — leave empty for an all-day item' : 'Set a due date first'}
              />
            </span>
          </label>
          <label class="field checkbox">
            <input
              aria-label="hard deadline"
              type="checkbox"
              bind:checked={pHard}
              onchange={() => setProp('hard', pHard ? 'true' : 'false')}
            />
            <span>Hard deadline</span>
          </label>
          <label class="field wide">
            <span>Title</span>
            <input
              aria-label="title"
              bind:value={pTitle}
              oninput={() => setPropDebounced('title', pTitle)}
              placeholder="optional title"
              spellcheck="false"
            />
          </label>
          <label class="field wide">
            <span>Tags</span>
            <input
              aria-label="tags"
              bind:value={pTags}
              oninput={() => setPropDebounced('tags', pTags)}
              placeholder="comma or space separated"
              spellcheck="false"
            />
          </label>
        </div>
      {/if}
      {#if isDiscussion}
        <!-- A discussion has no document body; its content is the thread below, always shown. -->
      {:else if isBoard}
        <Whiteboard body={note.body} theme={boardTheme} onSave={saveBoard} />
      {:else if editing}
        <div class="editor-wrap">
          {#snippet fmtButtons()}
            <button class="fmt-btn" title="Bold" aria-label="bold" onclick={() => wrapSel('**', '**', 'bold')}><b>B</b></button>
            <button class="fmt-btn" title="Italic" aria-label="italic" onclick={() => wrapSel('*', '*', 'italic')}><i>I</i></button>
            <button class="fmt-btn" title="Highlight" aria-label="highlight" onclick={() => wrapSel('==', '==', 'text')}>==</button>
            <button class="fmt-btn code" title="Code" aria-label="code" onclick={() => wrapSel('`', '`', 'code')}>{'</>'}</button>
            <div class="fmt-color">
              <button class="fmt-btn" title="Colour" aria-label="colour" aria-expanded={colorOpen} onclick={() => (colorOpen = !colorOpen)}>A<span class="caret">▾</span></button>
              {#if colorOpen}
                <ul class="fmt-colors" role="listbox" aria-label="colour token">
                  {#each TEXT_TOKENS as t (t)}
                    <li><button class="fmt-color-opt" data-token={t} onclick={() => wrapSel('[', `]{.${t}}`, 'text')}>{t}</button></li>
                  {/each}
                </ul>
              {/if}
            </div>
            <button class="fmt-btn" title="Link" aria-label="link" onclick={insertLink}>🔗</button>
            <div class="fmt-color">
              <button class="fmt-btn" title="Block format" aria-label="block format" aria-expanded={blockOpen} onclick={() => (blockOpen = !blockOpen)}>¶<span class="caret">▾</span></button>
              {#if blockOpen}
                <ul class="fmt-colors" role="listbox" aria-label="block format">
                  <li><button class="fmt-block-opt" onclick={() => heading(1)}>Heading 1</button></li>
                  <li><button class="fmt-block-opt" onclick={() => heading(2)}>Heading 2</button></li>
                  <li><button class="fmt-block-opt" onclick={() => heading(3)}>Heading 3</button></li>
                  <li><button class="fmt-block-opt" onclick={bullets}>• Bullet list</button></li>
                  <li><button class="fmt-block-opt" onclick={numbered}>1. Numbered list</button></li>
                  <li><button class="fmt-block-opt" onclick={quoteSel}>❝ Quote</button></li>
                  <li><button class="fmt-block-opt" onclick={calloutBlock}>▍ Callout</button></li>
                </ul>
              {/if}
            </div>
          {/snippet}
          {#if coarsePointer}
            <!-- Touch: a persistent bar above the editor — the float would hide behind Android's
                 system Cut/Copy menu. `pointerdown` prevented so a press keeps the selection. -->
            <div class="fmt-bar fmt-bar-static" role="toolbar" tabindex="-1" aria-label="format text" onpointerdown={(e) => e.preventDefault()}>
              {@render fmtButtons()}
            </div>
          {/if}
          <textarea
            class="editor"
            bind:this={editorEl}
            bind:value={draft}
            oninput={onInput}
            onkeydown={onEditorKeydown}
            onselect={onEditorSelect}
            onmouseup={onEditorSelect}
            onkeyup={onEditorSelect}
            ondragover={onDragOver}
            ondrop={onDrop}
            onblur={() => setTimeout(() => { closeSlash(); fmtBar = null; }, 120)}
            spellcheck="false"
            aria-label="note body (Markdown)"
          ></textarea>
          {#if adding}<span class="adding">Adding…</span>{/if}
          {#if slash.open && slash.results.length}
            <ul
              class="slash-menu"
              class:embedding={slash.embed}
              role="listbox"
              aria-label={slash.embed ? 'insert an embed' : 'insert a link or embed'}
              style="top: {slash.at.top}px; left: {slash.at.left}px"
            >
              {#each slash.results as r, i (r.id)}
                <li
                  role="option"
                  aria-selected={i === slash.active}
                  class:active={i === slash.active}
                  onmousedown={(e) => {
                    e.preventDefault();
                    chooseSlash(r, e.shiftKey);
                  }}
                >
                  <span class="slash-type" data-type={r.type}>{r.type}</span>
                  <span class="slash-title">{r.title ?? r.preview}</span>
                </li>
              {/each}
              <li class="slash-hint" aria-hidden="true">
                {#if slash.embed}
                  <kbd>//</kbd> embedding — tap to insert
                {:else}
                  <kbd>↵</kbd> link · <kbd>⇧↵</kbd> or <kbd>//</kbd> embed
                {/if}
              </li>
            </ul>
          {/if}
          {#if !coarsePointer && fmtBar && !slash.open}
            <!-- Desktop: a discrete bar that floats above the selection, only while text is selected. -->
            <div
              class="fmt-bar fmt-bar-float"
              role="toolbar"
              tabindex="-1"
              aria-label="format selection"
              style="top: {fmtBar.top}px; left: {fmtBar.left}px"
              onpointerdown={(e) => e.preventDefault()}
            >
              {@render fmtButtons()}
            </div>
          {/if}
        </div>
        <p class="editor-hint">
          Drag files in to attach · <kbd>/</kbd> to link a note or asset,
          <kbd>//</kbd> to embed one · <kbd>Ctrl</kbd>+<kbd>S</kbd> to save
        </p>
      {:else}
        <!-- Chips are built by render.ts, so one delegated listener beats
             re-binding per chip on every render. The handler only acts on a
             .note-chip, and each chip is a real <button> — so the keyboard path
             works natively and this stays a click-target shortcut, not the only
             way in.
             Double-click anywhere else here opens the editor; there is no Edit
             button any more. dblclick has no keyboard equivalent, so the view is
             focusable and Enter does the same job — otherwise dropping the button
             would leave the keyboard with no way in at all. -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="read"
          bind:this={content}
          tabindex="0"
          onclick={onReadClick}
          ondblclick={startEdit}
          onkeydown={(e) => {
            // Only when the view itself has focus — never when a chip inside it does.
            if (e.key === 'Enter' && e.target === content) {
              e.preventDefault();
              void openEditor();
            }
          }}
          title="Double-click to edit"
        ></div>
      {/if}

      <!-- Discussion. Inside the note's own pane rather than a pane of its own: a discussion
           is *about* a note, and two panes could drift apart on screen (panes are capped at 8
           anyway). Collapsed by default and fetched on open, so a note that nobody has
           discussed costs nothing to display. -->
      {#if backRefs.length}
        <!-- "Linked from": the reverse of the note references in this note's body, shown only when
             non-empty. Clicking one opens it in a pane, the same navigation as a chip. -->
        <section class="backlinks">
          <div class="backlinks-head">Linked from</div>
          <div class="backlinks-list">
            {#each backRefs as b (b.id)}
              <button class="backlink" onclick={() => onnavigate?.(b.id)} title={b.title ?? b.id}>
                {#if b.vault}<VaultBadge vault={b.vault} />{/if}
                <span class="backlink-title">{b.title || b.preview || b.id}</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}
      {#if note && !isBoard}
        <section class="discussion" class:is-discussion={isDiscussion}>
          {#if isDiscussion}
            <!-- The discussion IS the note; its title is the at-a-glance label, editable here
                 because a discussion has no separate edit mode. -->
            <input
              class="disc-title"
              value={note.title ?? ''}
              onchange={(e) => renameDiscussion(e.currentTarget.value)}
              placeholder="Discussion title"
              aria-label="discussion title"
            />
          {:else}
            <button class="disc-toggle" onclick={toggleDiscussion} aria-expanded={discOpen}>
              <span class="disc-caret" class:open={discOpen}>▸</span>
              Discussion{#if discCount > 0}<span class="disc-count">{discCount}</span>{/if}
            </button>
          {/if}
          {#if discOpen}
            {#each discMessages as m (m.id)}
              <!-- Indent from the server's `depth`: it is capped and cycle-guarded there, so a
                   hand-edited `reply_to` cannot push a row off-screen or hang this loop. -->
              <div class="disc-msg" style="margin-left: {m.depth * 1.1}rem">
                <div class="disc-meta">
                  {#if m.vault}<VaultBadge vault={m.vault} />{/if}
                  <EditedBy edit={lastEditFor(m.id)} />
                  <button class="disc-reply" onclick={() => startReply(m.id)}>Reply</button>
                </div>
                <p class="disc-body">{m.body}</p>
              </div>
            {/each}
            <div class="disc-compose">
              {#if replyTo && replyTo !== note?.id}
                <p class="disc-replying">
                  Replying to a message ·
                  <button class="disc-cancel" onclick={() => (replyTo = note?.id ?? '')}>to the note instead</button>
                </p>
              {/if}
              <textarea
                class="disc-input"
                bind:value={replyDraft}
                onkeydown={onReplyKeydown}
                placeholder="Add to the discussion…"
                aria-label="write a message"
              ></textarea>
              <div class="disc-actions">
                {#if discError}<span class="disc-error">{discError}</span>{/if}
                <button class="disc-send" disabled={!replyDraft.trim() || discBusy} onclick={sendReply}>
                  {discBusy ? 'Posting…' : 'Reply'}
                </button>
              </div>
            </div>
          {/if}
        </section>
      {/if}
    {:else if !error}
      <p class="loading">Loading…</p>
    {/if}
    {#if cellEdit}
      <!-- Tap-to-edit a table cell: a single-line input overlaid on the cell, prefilled with its
           *source* text. Enter or blur commits (patching just that cell); Escape cancels. -->
      <input
        class="cell-input"
        style="top: {cellEdit.top}px; left: {cellEdit.left}px; width: {cellEdit.width}px; height: {cellEdit.height}px"
        bind:value={cellEdit.value}
        aria-label="edit cell"
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            commitCell();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            cellEdit = null;
          }
        }}
        onblur={commitCell}
        use:autofocus
      />
    {/if}
    {#if calloutPick}
      <!-- Tap a callout's type badge to change its kind. A small menu of the closed callout
           vocabulary, anchored under the badge; dismissed by tapping outside or Escape. -->
      <ul
        class="callout-picker"
        role="listbox"
        aria-label="callout type"
        style="top: {calloutPick.top}px; left: {calloutPick.left}px"
        use:clickOutside={() => (calloutPick = null)}
      >
        {#each CALLOUT_OPTIONS as t (t)}
          <li>
            <button type="button" class="callout-opt callout-{t}" onclick={() => pickCallout(t)}>{t}</button>
          </li>
        {/each}
      </ul>
    {/if}
</article>

<style>
  /* Right-docked reading/editing sheet (Notion side-peek). The trail that holds this is a
     grid COLUMN of the app (App.svelte), no longer a modal overlay — so a pane fills its
     height (100%, not 100vh) and the trail column, not the viewport, sets the width. */
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: clamp(32rem, 42vw, 44rem);
    max-width: 100%;
    flex: none; /* a trail column keeps its width; the row scrolls instead */
    scroll-snap-align: end;
    background: var(--surface);
    box-shadow: var(--shadow-lg);
    overflow-y: auto;
    animation: sheet-in var(--dur-med) var(--ease);
  }
  /* A lone pane fills the trail column exactly (docked cap, or the whole content area
     when wide); a second pane joining makes them fall back to their own width and scroll. */
  .panel.solo {
    width: 100%;
  }
  /* Pure full-screen board: the pane covers the whole viewport, over all app chrome — nothing but
     the canvas. The note header is hidden; only the floating ✕ (and the Back button) remain. */
  .panel.board-full {
    position: fixed;
    inset: 0;
    z-index: 200;
    width: 100vw;
    height: 100%;
    height: 100dvh; /* excludes the phone's URL/nav bars where supported */
    max-width: none;
    overflow: hidden;
    box-shadow: none;
    animation: none;
  }
  .panel.board-full > header {
    display: none;
  }
  .board-exit {
    position: fixed;
    top: max(env(safe-area-inset-top, 0px), var(--space-2));
    right: max(env(safe-area-inset-right, 0px), var(--space-2));
    z-index: 210;
    display: grid;
    place-items: center;
    width: 2.4rem;
    height: 2.4rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-elevated);
    color: var(--text);
    font-size: 1rem;
    cursor: pointer;
    opacity: 0.85;
  }
  .board-exit:hover {
    opacity: 1;
  }
  @keyframes sheet-in {
    from {
      transform: translateX(2rem);
      opacity: 0;
    }
    to {
      transform: none;
      opacity: 1;
    }
  }
  /* Full screen: a *solo* pane fills the viewport. The reading column below stays
     capped at --measure and centered, so prose is still comfortable. Once a
     second pane joins the trail, panes fall back to their column width — the
     whole point of following a link is seeing where you came from. */
  .panel.wide.solo {
    width: 100%;
    height: 100%;
    max-width: none;
    animation-name: page-in;
  }
  @keyframes page-in {
    from {
      transform: translateY(1rem);
      opacity: 0;
    }
    to {
      transform: none;
      opacity: 1;
    }
  }
  header {
    position: sticky;
    top: 0;
    z-index: 2;
    /* Two stacked rows, not one. See the markup: the title owns the first line. */
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  /* While editing, the header doubles as "Done": clicking its chrome (title, identity line —
     anything but a control) flushes the draft and returns to the read view, the intuitive twin
     of Ctrl+S / Esc. The accent underline is the mode signal; the pointer cursor the invitation. */
  header.editing {
    cursor: pointer;
    border-bottom-color: var(--accent);
  }
  .title-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }
  /* The controls wrap rather than compress: on a narrow pane a second line of buttons is
     readable, whereas eight items crushed onto one is what this change exists to undo. */
  .control-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .type {
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    padding: 0.05rem 0.5rem;
  }
  h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
    /* One line, elided — a very long title must not push the header into a paragraph. */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .edit {
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: var(--text-xs);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
  }
  .edit:hover {
    border-color: var(--accent);
  }
  /* Destructive actions: quiet danger tint on the outline, filled on the final
     confirm button so the irreversible click is unmistakable. */
  .edit.danger {
    color: var(--danger-fg);
  }
  .edit.danger:hover {
    border-color: var(--danger-fg);
  }
  .edit.danger.solid {
    background: var(--danger-fg);
    border-color: var(--danger-fg);
    color: var(--danger-bg);
    font-weight: 600;
  }
  /* The two-step delete confirmation strip. */
  .confirm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background: var(--danger-bg);
    color: var(--danger-fg);
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--border);
  }
  .confirm-actions {
    display: flex;
    gap: var(--space-2);
    flex: none;
  }
  /* The "Copy to another vault" popover: a warning, an opt-in, and the vault targets.
     Reuses .edit button styling; no new modal system (it's an inline strip like .confirm). */
  .copy-pop {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
  }
  .copy-warn {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }
  .copy-opt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text);
  }
  .copy-targets {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  /* The extra confirm for carrying the files — warning-coloured, reversible via Cancel. */
  .copy-danger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--danger-bg);
    color: var(--danger-fg);
    border: 1px solid var(--danger-fg);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    line-height: 1.4;
  }
  /* The post-copy Undo strip — a copy is sensitive, so it stays visibly reversible. */
  .copy-undo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background: var(--ok-bg);
    color: var(--ok-fg);
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--border);
  }

  /* Transient success line (copied a file in), styled like App's .banner.notice. */
  .note-notice {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    background: var(--ok-bg);
    color: var(--ok-fg);
    font-size: var(--text-sm);
  }
  .capture {
    position: relative;
  }
  .capture-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    z-index: 5;
    min-width: 12rem;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    box-shadow: var(--shadow-lg);
  }
  .capture-menu button {
    display: block;
    width: 100%;
    text-align: left;
    /* The touch target both platform guidelines ask for — this menu exists for phones. */
    min-height: 2.75rem;
    padding: var(--space-2);
    background: none;
    border: none;
    border-radius: var(--radius-2, 6px);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .capture-menu button:hover {
    background: var(--surface-hover);
  }
  /* The `＋`-options button and its window share one relative wrapper (the `.capture` shape),
     so the window is positioned against the *button*, not the header edge. */
  .options {
    position: relative;
    display: inline-flex;
  }
  /* The note-options window: a card anchored directly under the `＋`, not a dropdown list.
     `left: 0` hangs it from the button's left edge, opening rightward, so it sits beside the `＋`
     at any scroll offset — anchored to the header/panel it drifted to a corner of the pane (the
     "weird places" bug). Clamped so a narrow phone pane never pushes it off-screen. */
  .options-window {
    position: absolute;
    left: 0;
    top: calc(100% + 4px);
    z-index: 20;
    min-width: 13rem;
    max-width: min(20rem, 88vw);
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
  }
  .options-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-1) var(--space-2) var(--space-2);
    color: var(--text-subtle);
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .opt-close {
    background: none;
    border: 0;
    color: var(--text-subtle);
    font: inherit;
    cursor: pointer;
    line-height: 1;
  }
  /* Full-width rows with the ~44px touch target both platforms ask for — this window is for phones. */
  .opt {
    display: block;
    width: 100%;
    text-align: left;
    min-height: 2.75rem;
    padding: var(--space-2);
    background: none;
    border: none;
    border-radius: var(--radius-2, 6px);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .opt:hover {
    background: var(--surface-hover);
  }
  .opt.danger {
    color: var(--danger-fg);
  }
  /* Hidden, never `display: none`: a display-none input cannot be opened by `.click()` in
     every engine, and this one is only ever driven programmatically. */
  .capture-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  .icon-toggle {
    display: grid;
    place-items: center;
    width: 1.9rem;
    height: 1.9rem;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 1rem;
    cursor: pointer;
  }
  .icon-toggle:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  /* The Obsidian/Notion-style properties form, shown in edit mode above the body. */
  .props {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-2) var(--space-4);
    padding: var(--space-4) var(--space-5) var(--space-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-muted);
    min-width: 0;
  }
  .field.wide {
    grid-column: 1 / -1;
  }
  /* Date + optional time: the date takes the room it needs, the time is a narrow
     refinement beside it, and it dims until there's a date to attach it to. */
  .when {
    display: flex;
    gap: var(--space-1);
    min-width: 0;
  }
  .when input[type='date'] {
    flex: 1;
    min-width: 0;
  }
  .when input[type='time'] {
    flex: 0 0 auto;
    width: 6.5em;
  }
  .when input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .field.checkbox {
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    align-self: end;
    padding-bottom: var(--space-1);
  }
  .field input {
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-size: var(--text-sm);
  }
  .field.checkbox input {
    width: auto;
  }
  .close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1rem;
    cursor: pointer;
  }
  .close:hover {
    color: var(--text);
  }
  .editor-wrap {
    position: relative;
    flex: 1;
    display: flex;
  }
  .editor {
    width: 100%;
    min-height: 22rem;
    resize: vertical;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-5);
    border: none;
    background: transparent;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: 1.6;
  }
  .editor:focus-visible {
    outline: none;
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .adding {
    position: absolute;
    top: var(--space-2);
    right: var(--space-4);
    font-size: var(--text-xs);
    color: var(--accent);
  }
  /* Anchored to the caret: `top`/`left` are set inline from caretXY on open, so
     the menu appears under what you are typing. The values here are only the
     fallback for a browser that never ran the measurement. */
  .slash-menu {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 60;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    min-width: 16rem;
    max-height: 14rem;
    overflow-y: auto;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  /* Embed mode (`//`) gets an accent frame so it's clear a tap will embed, not link. */
  .slash-menu.embedding {
    border-color: var(--accent);
  }
  /* The formatting toolbar. Two presentations share this base: a discrete float on desktop, a
     persistent row on touch. Kept small — a discrete presence, never a big band across the editor. */
  .fmt-bar {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  /* Desktop: floats above the selection, only while selecting. */
  .fmt-bar-float {
    position: absolute;
    z-index: 55;
    box-shadow: var(--shadow-md);
  }
  /* Touch: a persistent strip above the editor; scrolls sideways on a narrow phone. */
  .fmt-bar-static {
    margin-bottom: var(--space-1);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .fmt-bar-static::-webkit-scrollbar {
    display: none;
  }
  .fmt-btn {
    flex: none;
  }
  .fmt-btn {
    display: grid;
    place-items: center;
    min-width: 1.9rem;
    height: 1.9rem;
    padding: 0 0.35rem;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .fmt-btn:hover {
    background: var(--surface-hover);
  }
  .fmt-btn.code {
    font-family: var(--mono, monospace);
    font-size: var(--text-xs);
  }
  .fmt-btn .caret {
    font-size: 0.6em;
    margin-left: 1px;
  }
  .fmt-color {
    position: relative;
    display: inline-flex;
  }
  .fmt-colors {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 56;
    min-width: 6rem;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  .fmt-color-opt {
    display: block;
    width: 100%;
    text-align: left;
    min-height: 2rem;
    padding: 0.1rem 0.4rem;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: inherit;
    font-weight: 600;
    text-transform: capitalize;
    cursor: pointer;
  }
  .fmt-color-opt:hover {
    background: var(--surface-hover);
  }
  .fmt-block-opt {
    display: block;
    width: 100%;
    text-align: left;
    min-height: 2rem;
    padding: 0.1rem 0.5rem;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
  }
  .fmt-block-opt:hover {
    background: var(--surface-hover);
  }
  .fmt-color-opt[data-token='accent'] {
    color: var(--accent);
  }
  .fmt-color-opt[data-token='info'] {
    color: var(--tint-a);
  }
  .fmt-color-opt[data-token='ok'] {
    color: var(--tint-b);
  }
  .fmt-color-opt[data-token='warn'] {
    color: var(--tint-c);
  }
  .fmt-color-opt[data-token='muted'] {
    color: var(--muted);
  }
  /* The tap-to-edit table cell input, overlaid on the cell (fixed, to the viewport). */
  .cell-input {
    position: fixed;
    z-index: 55;
    box-sizing: border-box;
    min-height: 1.8rem;
    padding: 0 0.4rem;
    background: var(--surface-elevated);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
  }
  /* The callout-type picker: a small menu anchored (fixed, to the viewport) under the tapped badge. */
  .callout-picker {
    position: fixed;
    z-index: 60;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    min-width: 8rem;
    max-width: min(14rem, 90vw);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  .callout-opt {
    display: block;
    width: 100%;
    text-align: left;
    min-height: 2.25rem;
    padding: var(--space-1) var(--space-2);
    background: none;
    border: none;
    border-left: 3px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    text-transform: capitalize;
    cursor: pointer;
  }
  .callout-opt:hover {
    background: var(--surface-hover);
  }
  /* Each option carries its own accent, so the menu previews what the callout will look like. */
  .callout-opt.callout-note,
  .callout-opt.callout-info {
    border-left-color: var(--tint-a);
  }
  .callout-opt.callout-tip {
    border-left-color: var(--tint-b);
  }
  .callout-opt.callout-warning {
    border-left-color: var(--tint-c);
  }
  .callout-opt.callout-danger {
    border-left-color: var(--accent);
  }
  .callout-opt.callout-quote {
    border-left-color: var(--muted);
  }
  .slash-menu li {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    color: var(--text);
    cursor: pointer;
    overflow: hidden;
    white-space: nowrap;
  }
  .slash-menu li.active {
    background: var(--surface-hover);
  }
  /* The menu lists notes and assets together, so each row says which it is —
     that's what tells you whether you're about to embed or link. */
  .slash-type {
    flex: none;
    font-size: var(--text-xs);
    color: var(--text-subtle);
    text-transform: lowercase;
  }
  .slash-title {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* A non-interactive footer teaching the one modifier: Enter links, Shift+Enter embeds. */
  .slash-hint {
    cursor: default;
    margin-top: var(--space-1);
    padding-top: var(--space-1);
    border-top: 1px solid var(--border);
    color: var(--text-subtle);
    font-size: var(--text-xs);
  }
  .slash-hint:hover {
    background: none;
  }
  .slash-hint kbd {
    font-size: inherit;
  }
  /* ── Discussion ─────────────────────────────────────────────────────────────────────
     Deliberately quieter than the note body: this is commentary *about* the note, and it
     sits below it, so it must not compete with the thing it is about. */
  .backlinks {
    border-top: 1px solid var(--border);
    padding: var(--space-2) var(--space-5) var(--space-3);
  }
  .backlinks-head {
    margin-bottom: var(--space-2);
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-subtle);
  }
  .backlinks-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .backlink {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    max-width: 100%;
    padding: var(--space-1) var(--space-2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .backlink:hover {
    border-color: var(--accent);
  }
  .backlink-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .discussion {
    border-top: 1px solid var(--border);
    padding: var(--space-2) var(--space-5) var(--space-4);
  }
  /* A discussion pane is the thread, so it has no top border (nothing above it to divide from). */
  .discussion.is-discussion {
    border-top: 0;
  }
  .disc-title {
    width: 100%;
    margin: 0 0 var(--space-2);
    padding: var(--space-1) 0;
    background: none;
    border: 0;
    border-bottom: 1px solid transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .disc-title:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  .disc-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) 0;
    background: none;
    border: 0;
    color: var(--text-subtle);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: left;
  }
  .disc-toggle:hover {
    color: var(--text);
  }
  .disc-caret {
    display: inline-block;
    transition: transform 120ms ease;
  }
  .disc-caret.open {
    transform: rotate(90deg);
  }
  .disc-count {
    margin-left: var(--space-1);
    padding: 0 0.4em;
    border-radius: 999px;
    background: var(--surface-hover);
    font-size: var(--text-xs);
  }
  .disc-msg {
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border-subtle, var(--border));
  }
  .disc-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  .disc-reply {
    margin-left: auto;
    background: none;
    border: 0;
    color: var(--text-subtle);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .disc-reply:hover {
    color: var(--accent, var(--text));
    text-decoration: underline;
  }
  .disc-body {
    margin: var(--space-1) 0 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .disc-compose {
    margin-top: var(--space-3);
  }
  .disc-replying {
    margin: 0 0 var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  .disc-cancel {
    background: none;
    border: 0;
    padding: 0;
    color: var(--text-subtle);
    font: inherit;
    font-size: var(--text-xs);
    text-decoration: underline;
    cursor: pointer;
  }
  .disc-input {
    width: 100%;
    min-height: 3.5rem;
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    resize: vertical;
  }
  .disc-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    justify-content: flex-end;
    margin-top: var(--space-1);
  }
  .disc-error {
    margin-right: auto;
    font-size: var(--text-xs);
    color: var(--danger, #dc2626);
  }
  .disc-send {
    /* 2.75rem is this app's touch target on a coarse pointer — same rule as every other
       primary control here, so a thumb can reach it on the phone. */
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--surface-hover);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .disc-send:disabled {
    opacity: 0.5;
    cursor: default;
  }
  @media (pointer: coarse) {
    .disc-send {
      min-height: 2.75rem;
    }
  }
  .editor-hint {
    margin: 0;
    padding: var(--space-1) var(--space-5) var(--space-3);
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  .editor-hint kbd {
    font-family: var(--font-mono);
    background: var(--surface-hover);
    border-radius: 4px;
    padding: 0 0.3em;
  }
  /* Reading column capped at the measure and centered in wide/page mode. */
  .read {
    width: 100%;
    max-width: var(--measure);
    margin: 0 auto;
    padding: var(--space-4) var(--space-5) var(--space-6);
    box-sizing: border-box;
    color: var(--text);
    /* Fill the panel below the text, not just the text: `.panel` is a 100vh flex
       column, so without this a short note leaves a tall dead zone that looks
       like the note but isn't — double-clicking there would hit nothing. Basis
       stays `auto` so long content still sizes itself and the panel scrolls. */
    flex: 1 0 auto;
  }
  .loading,
  .err {
    padding: var(--space-5);
    color: var(--text-muted);
  }
  .err {
    color: var(--danger-fg);
  }
</style>

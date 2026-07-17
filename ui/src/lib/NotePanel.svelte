<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import {
    getNote,
    updateBody,
    setProperty,
    deleteNote,
    resolveAsset as ipcResolveAsset,
    assetStatus,
    openExternal,
    ingestFile,
    search,
    recent,
  } from './ipc';
  import { renderInto, type ResolvedAsset, type ResolvedNote } from './render';
  import { parseStamp, toStamp } from './stamp';
  import { caretXY, clamp } from './caret';
  import { countOf, nthIndexOf } from './locate';
  import StatusChip from './StatusChip.svelte';
  import Whiteboard from './Whiteboard.svelte';
  import type { NoteDetail, ObjectMeta } from './types';

  let {
    id,
    onclose,
    onsaved,
    onnavigate,
    ontogglewide,
    wide = true,
    solo = true,
    statuses = [],
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
  // Deleting is destructive + irreversible, so the button arms a confirm strip
  // (a second, deliberate click) rather than firing on the first press.
  let confirmingDelete = $state(false);
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
    at: { top: number; left: number };
  };
  let slash = $state<SlashState>({
    open: false,
    from: -1,
    query: '',
    results: [],
    active: 0,
    at: { top: 0, left: 0 },
  });
  let slashTimer: ReturnType<typeof setTimeout> | undefined;
  // The pane's root, so a global key (Ctrl+S) can tell whether *this* pane in the
  // trail is the one being typed in.
  let paneEl = $state<HTMLElement | undefined>(undefined);

  // Object URLs minted for inline assets, revoked when the note changes or the
  // panel closes so the blobs don't leak.
  let assetUrls: string[] = [];
  function revokeAssets() {
    for (const u of assetUrls) URL.revokeObjectURL(u);
    assetUrls = [];
  }
  onDestroy(revokeAssets);

  // Fetch an asset's bytes + sniffed MIME and hand render.ts a typed object URL,
  // so it can pick the right inline element (image / PDF / video / audio). null
  // (missing blob, or the browser/test mock) → the inline "not available"
  // placeholder; media absence is a warning, never a broken pane.
  async function resolveAsset(ref: string): Promise<ResolvedAsset | null> {
    try {
      const status = await assetStatus(ref);
      if (!status.has_blob) return null;
      const buf = await ipcResolveAsset(ref, 'full');
      if (!buf || buf.byteLength === 0) return null;
      const mime = status.mime ?? '';
      const url = URL.createObjectURL(new Blob([buf], mime ? { type: mime } : undefined));
      assetUrls.push(url);
      return { url, mime };
    } catch {
      return null;
    }
  }

  $effect(() => {
    note = null;
    editing = false;
    revokeAssets();
    getNote(id)
      .then((n) => {
        note = n;
        draft = n?.body ?? '';
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
      renderInto(content, note.body, resolveAsset, resolveNote).catch((e) => (error = String(e)));
    }
  });

  // A `note:` chip needs the target's live title/status. `get` already returns
  // them (it over-fetches the body, which a chip ignores — not worth a second IPC
  // command until it measures). A missing note resolves to null and the chip
  // degrades to a placeholder.
  async function resolveNote(refId: string): Promise<ResolvedNote | null> {
    const n = await getNote(refId);
    return n && { id: n.id, type: n.type, title: n.title, status: n.status };
  }

  // One delegated listener for every chip in the pane — chips are created by
  // render.ts, so binding per-chip would mean re-binding on every render.
  function onReadClick(e: MouseEvent) {
    const chip = (e.target as HTMLElement | null)?.closest<HTMLElement>('.note-chip');
    const target = chip?.dataset.noteId;
    if (target) onnavigate?.(target);
  }

  // Debounced save: typing stops -> 500 ms -> atomic write via update_body.
  // Also re-evaluate the slash-menu trigger against the new caret.
  function onInput() {
    saved = false;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 500);
    detectSlash();
  }

  async function save() {
    if (!note) return;
    try {
      await updateBody(note.id, draft);
      note = { ...note, body: draft };
      saved = true;
      onsaved?.();
    } catch (e) {
      error = String(e);
    }
  }

  // A board note carries `view: board`; its body is an Excalidraw scene (JSON),
  // edited on a canvas instead of as text. Detection is a plain property check —
  // no new Kind (the model resists that).
  let isBoard = $derived(note?.props?.view === 'board');
  let boardTheme: 'dark' | 'light' = $derived(
    document.documentElement.dataset.theme === 'light' ? 'light' : 'dark',
  );

  // The canvas persists the same way the textarea does: write the body bytes.
  async function saveBoard(json: string) {
    if (!note) return;
    try {
      await updateBody(note.id, json);
      note = { ...note, body: json };
      onsaved?.();
    } catch (e) {
      error = String(e);
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

  // Double-click the read view to edit it — there is no Edit button any more.
  async function startEdit(e: MouseEvent) {
    // Skip the targets that already mean something: a reference chip navigates, a
    // link follows, media has its own controls, and double-clicking to select a
    // word inside them should not throw you into the editor.
    if ((e.target as HTMLElement | null)?.closest('.note-chip, a, button, video, audio, iframe')) {
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

  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes('Files')) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    }
  }
  async function onDrop(e: DragEvent) {
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (!files.length) return;
    e.preventDefault();
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
      e.preventDefault();
      chooseSlash(slash.results[slash.active]);
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
    if (i !== 0 && !/\s/.test(draft[i - 1])) return closeSlash();
    const query = draft.slice(i + 1, caret);
    if (/\s/.test(query)) return closeSlash();
    slash = { ...slash, open: true, from: i, query, active: 0, at: slashAnchor(el, i) };
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
  async function chooseSlash(meta: ObjectMeta) {
    const el = editorEl;
    if (!el) return;
    const caret = el.selectionStart;
    const ref = refFor(meta);
    draft = draft.slice(0, slash.from) + ref + draft.slice(caret);
    closeSlash();
    onInput();
    await tick();
    const pos = slash.from + ref.length;
    el.selectionStart = el.selectionEnd = pos;
    el.focus();
  }
</script>

<svelte:window onkeydown={onPaneKey} />

<article class="panel" class:wide class:solo bind:this={paneEl}>
    <header>
      {#if note && note.type === 'asset'}<span class="type" data-type={note.type}>{note.type}</span>{/if}
      <h2>{note?.title ?? 'note'}</h2>
      {#if note}
        <!-- Always visible, no edit mode needed: rotating status is the most
             frequent edit a note gets. Typing a brand-new value is Details' job. -->
        <StatusChip
          status={note.status}
          {statuses}
          onchange={(next) => setProp('status', next ?? '')}
        />
      {/if}
      {#if note && note.type === 'asset' && note.assets.length}
        <button
          class="edit"
          onclick={() => openExternal(note!.assets[0]).catch((e) => (error = String(e)))}
        >
          Open
        </button>
      {/if}
      {#if note}
        <!-- The button is the discoverable way in and stays on every note.
             Double-clicking the read view is the same action without the trip to
             the header (a board's canvas owns double-click, so there the button is
             the only way — hence "Details" rather than "Edit"). -->
        <button class="edit" onclick={toggleEdit}>
          {#if isBoard}
            {editing ? 'Done' : 'Details'}
          {:else}
            {editing ? (saved ? 'Done' : 'Saving…') : 'Edit'}
          {/if}
        </button>
        <button class="edit danger" onclick={() => (confirmingDelete = true)} aria-label="delete note" title="Delete this note">
          Delete
        </button>
      {/if}
      <button class="icon-toggle" onclick={ontogglewide} aria-pressed={wide} aria-label="toggle full screen" title={wide ? 'Exit full screen' : 'Full screen'}>
        {wide ? '⤡' : '⤢'}
      </button>
      <button class="close" onclick={onclose} aria-label="close">✕</button>
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
      {#if isBoard}
        <Whiteboard body={note.body} theme={boardTheme} onSave={saveBoard} />
      {:else if editing}
        <div class="editor-wrap">
          <textarea
            class="editor"
            bind:this={editorEl}
            bind:value={draft}
            oninput={onInput}
            onkeydown={onEditorKeydown}
            ondragover={onDragOver}
            ondrop={onDrop}
            onblur={() => setTimeout(closeSlash, 120)}
            spellcheck="false"
            aria-label="note body (Markdown)"
          ></textarea>
          {#if adding}<span class="adding">Adding…</span>{/if}
          {#if slash.open && slash.results.length}
            <ul
              class="slash-menu"
              role="listbox"
              aria-label="insert a link"
              style="top: {slash.at.top}px; left: {slash.at.left}px"
            >
              {#each slash.results as r, i (r.id)}
                <li
                  role="option"
                  aria-selected={i === slash.active}
                  class:active={i === slash.active}
                  onmousedown={(e) => {
                    e.preventDefault();
                    chooseSlash(r);
                  }}
                >
                  <span class="slash-type" data-type={r.type}>{r.type}</span>
                  <span class="slash-title">{r.title ?? r.preview}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
        <p class="editor-hint">
          Drag files in to attach · type <kbd>/</kbd> to link a note or asset ·
          <kbd>Ctrl</kbd>+<kbd>S</kbd> to save
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
    {:else if !error}
      <p class="loading">Loading…</p>
    {/if}
</article>

<style>
  /* Right-docked reading/editing sheet (Notion side-peek). The overlay and
     backdrop live in App.svelte — a pane is one column of a trail, and only the
     trail as a whole is modal. */
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: clamp(32rem, 42vw, 44rem);
    max-width: 100%;
    flex: none; /* a trail column keeps its width; the row scrolls instead */
    scroll-snap-align: end;
    background: var(--surface);
    border-left: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    overflow-y: auto;
    animation: sheet-in var(--dur-med) var(--ease);
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
    width: 100vw;
    height: 100vh;
    max-width: none;
    border: none;
    border-radius: 0;
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
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
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
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
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
  /* Transient success line (copied a file in), styled like App's .banner.notice. */
  .note-notice {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    background: var(--ok-bg);
    color: var(--ok-fg);
    font-size: var(--text-sm);
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

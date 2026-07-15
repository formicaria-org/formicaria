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
  } from './ipc';
  import { renderInto, type ResolvedAsset } from './render';
  import Whiteboard from './Whiteboard.svelte';
  import type { NoteDetail, ObjectMeta } from './types';

  let {
    id,
    onclose,
    onsaved,
    statuses = [],
    startEditing = false,
  }: {
    id: string;
    onclose: () => void;
    onsaved?: () => void;
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
  // Full screen by default (the preferred reading/writing mode); the toggle
  // shrinks to a docked side-sheet, and the choice is remembered per-browser like
  // the theme. Anything other than the stored '0' (incl. unset) means full screen.
  // Guard storage access — it's absent in the test env and in private mode.
  function readWidePref(): boolean {
    try {
      return localStorage.getItem('fm-note-wide') !== '0';
    } catch {
      return true;
    }
  }
  let wide = $state(readWidePref());
  function toggleWide() {
    wide = !wide;
    try {
      localStorage.setItem('fm-note-wide', wide ? '1' : '0');
    } catch {
      /* private mode / storage disabled — the default (full screen) still applies */
    }
  }
  // Deleting is destructive + irreversible, so the button arms a confirm strip
  // (a second, deliberate click) rather than firing on the first press.
  let confirmingDelete = $state(false);
  let draft = $state('');
  let saved = $state(true);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Editable property fields, initialized from the note when it loads. Each maps
  // to exactly what `apply_property` (via set_property) accepts: due is
  // YYYY-MM-DD, hard is a bool, tags are comma/space separated. There is no type
  // field — notes are differentiated by tags, and `asset` is set only by ingest.
  let pTitle = $state('');
  let pStatus = $state('');
  let pStart = $state('');
  let pDue = $state('');
  let pHard = $state(false);
  let pTags = $state('');
  let propTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  // The body editor, for caret-based insertion (drag-drop + slash-menu).
  let editorEl = $state<HTMLTextAreaElement | undefined>(undefined);
  let adding = $state(false);

  // Slash-menu (Notion-style `/` → insert an asset) state.
  type SlashState = { open: boolean; from: number; query: string; results: ObjectMeta[]; active: number };
  let slash = $state<SlashState>({ open: false, from: -1, query: '', results: [], active: 0 });
  let slashTimer: ReturnType<typeof setTimeout> | undefined;

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
          pStart = n.start ?? '';
          pDue = n.due ?? '';
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
      renderInto(content, note.body, resolveAsset).catch((e) => (error = String(e)));
    }
  });

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

  // The Markdown reference for an ingested asset. The URL half is pure hex, so
  // only the alt text can break link syntax — escape `\`, `[`, `]`.
  function assetRef(meta: ObjectMeta): string {
    const alt = (meta.title ?? 'asset').replace(/\\/g, '\\\\').replace(/[[\]]/g, '\\$&');
    const hash = meta.assets[0]?.replace(/^sha256:/, '') ?? '';
    return `![${alt}](asset:sha256-${hash})`;
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
        const meta = await ingestFile(f);
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
  // space dismisses. Debounced FTS on the query, filtered to assets.
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
    slash = { ...slash, open: true, from: i, query, active: 0 };
    clearTimeout(slashTimer);
    slashTimer = setTimeout(runSlashSearch, 150);
  }
  async function runSlashSearch() {
    const q = slash.query.trim();
    if (!q) {
      slash = { ...slash, results: [] };
      return;
    }
    try {
      const all = await search(q);
      slash = { ...slash, results: all.filter((n) => n.type === 'asset').slice(0, 8), active: 0 };
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
    const ref = assetRef(meta);
    draft = draft.slice(0, slash.from) + ref + draft.slice(caret);
    closeSlash();
    onInput();
    await tick();
    const pos = slash.from + ref.length;
    el.selectionStart = el.selectionEnd = pos;
    el.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" class:wide>
  <button class="backdrop" aria-label="close note" onclick={onclose}></button>
  <article class="panel" class:wide>
    <header>
      {#if note && note.type === 'asset'}<span class="type" data-type={note.type}>{note.type}</span>{/if}
      <h2>{note?.title ?? 'note'}</h2>
      {#if note && note.type === 'asset' && note.assets.length}
        <button
          class="edit"
          onclick={() => openExternal(note!.assets[0]).catch((e) => (error = String(e)))}
        >
          Open
        </button>
      {/if}
      {#if note}
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
      <button class="icon-toggle" onclick={toggleWide} aria-pressed={wide} aria-label="toggle full screen" title={wide ? 'Exit full screen' : 'Full screen'}>
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
            <input
              aria-label="start"
              type="date"
              bind:value={pStart}
              onchange={() => setProp('start', pStart)}
            />
          </label>
          <label class="field">
            <span>Due</span>
            <input aria-label="due" type="date" bind:value={pDue} onchange={() => setProp('due', pDue)} />
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
            <ul class="slash-menu" role="listbox" aria-label="insert asset">
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
                  {r.title ?? r.preview}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
        <p class="editor-hint">Drag files in to attach · type <kbd>/</kbd> to insert an asset</p>
      {:else}
        <div class="read" bind:this={content}></div>
      {/if}
    {:else if !error}
      <p class="loading">Loading…</p>
    {/if}
  </article>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: flex-end;
    z-index: 50;
  }
  .overlay.wide {
    justify-content: stretch;
    align-items: stretch;
    padding: 0;
  }
  /* A full-area button behind the panel: clicking outside closes, with no
     stopPropagation and no listeners on non-interactive elements. */
  .backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.5);
    cursor: default;
  }
  /* Right-docked reading/editing sheet (Notion side-peek). */
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: clamp(32rem, 42vw, 44rem);
    max-width: 100%;
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
  /* Full screen: the panel fills the viewport. The reading column below stays
     capped at --measure and centered, so prose is still comfortable. */
  .panel.wide {
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
  .slash-menu {
    position: absolute;
    top: 2.4rem;
    left: var(--space-5);
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
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    color: var(--text);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .slash-menu li.active {
    background: var(--surface-hover);
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

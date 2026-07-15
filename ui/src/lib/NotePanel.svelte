<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import {
    getNote,
    updateBody,
    setProperty,
    resolveAsset as ipcResolveAsset,
    assetStatus,
    openExternal,
    ingestFile,
    search,
  } from './ipc';
  import { renderInto, type ResolvedAsset } from './render';
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
  let editing = $state(false);
  // Docked side-sheet by default; "Open wide" expands to a centered page (Craft/Notion).
  let wide = $state(false);
  let draft = $state('');
  let saved = $state(true);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Editable property fields, initialized from the note when it loads. Each maps
  // to exactly what `apply_property` (via set_property) accepts — see the value
  // formats in the plan: type is a fixed kind, due is YYYY-MM-DD, hard is a bool,
  // tags are comma/space separated.
  const KINDS = ['note', 'task', 'meeting', 'asset'];
  let pType = $state('note');
  let pTitle = $state('');
  let pStatus = $state('');
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
          pType = n.type;
          pTitle = n.title ?? '';
          pStatus = n.status ?? '';
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
    if (key === 'type') note = { ...note, type: value };
    else if (key === 'title') note = { ...note, title: value || null };
    else if (key === 'status') note = { ...note, status: value || null };
    else if (key === 'due') note = { ...note, due: value || null };
    else if (key === 'hard') note = { ...note, hard: value === 'true' };
    else if (key === 'tags')
      note = { ...note, tags: value ? value.split(/[,\s]+/).filter(Boolean) : [] };
  }

  async function toggleEdit() {
    if (editing) await save(); // leaving edit mode flushes any pending change
    editing = !editing;
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
    try {
      for (const f of files) {
        const meta = await ingestFile(f);
        await insertAtCaret(assetRef(meta) + '\n');
      }
      onsaved?.();
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
      {#if note}<span class="type" data-type={note.type}>{note.type}</span>{/if}
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
          {editing ? (saved ? 'Done' : 'Saving…') : 'Edit'}
        </button>
      {/if}
      <button class="icon-toggle" onclick={() => (wide = !wide)} aria-pressed={wide} aria-label="open wide" title={wide ? 'Dock to the side' : 'Open wide'}>
        {wide ? '⇥' : '⤢'}
      </button>
      <button class="close" onclick={onclose} aria-label="close">✕</button>
    </header>
    {#if error}
      <p class="err">{error}</p>
    {/if}
    {#if note}
      {#if editing}
        <div class="props">
          <label class="field">
            <span>Type</span>
            <select aria-label="type" bind:value={pType} onchange={() => setProp('type', pType)}>
              {#each KINDS as k (k)}<option value={k}>{k}</option>{/each}
            </select>
          </label>
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
    justify-content: center;
    align-items: flex-start;
    padding: 4vh var(--space-4);
    overflow-y: auto;
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
  /* "Open wide": a centered page instead of a side dock. */
  .panel.wide {
    height: auto;
    max-height: 92vh;
    width: min(60rem, 100%);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
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
  .field input,
  .field select {
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

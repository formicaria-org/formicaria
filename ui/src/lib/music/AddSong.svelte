<script lang="ts">
  // **Add a song** (`decisions.md#ui` `#data`, 2026-11-13, *a sixth group, Music*): the group's
  // plus. Each chosen file is uploaded like any attachment, and what its tags said — kept on its
  // asset note by `fm-core::audio` — fills the song's form. The person corrects and saves; a song
  // is a note with `log: music` whose body embeds the audio, so it plays wherever it is opened.
  //
  // A file whose audio a song already holds is not added twice. Nothing is looked up online here:
  // that is a press of its own, in a later phase.
  import Icon from '../Icon.svelte';
  import ManualLink from '../ManualLink.svelte';
  import { ingestFile, musicLibrary } from '../ipc';
  import { createNotesUndoable } from '../undo.svelte';
  import type { NoteDraft, ObjectMeta } from '../types';
  import { formOf, fromName, NUMBER, songDraft, TEXT, type Form } from './songDraft';

  interface Props {
    onclose: () => void;
    onsaved: () => void;
  }
  let { onclose, onsaved }: Props = $props();

  interface Item {
    key: number;
    name: string;
    state: 'reading' | 'ready' | 'failed' | 'known';
    why?: string;
    asset?: ObjectMeta;
    form: Form;
    open: boolean;
  }
  const LABEL: Record<keyof Form, string> = {
    title: 'Title',
    artist: 'Artist',
    album: 'Album',
    album_artist: 'Album artist',
    track: 'Track',
    disc: 'Disc',
    year: 'Year',
    genre: 'Genre',
  };

  let items = $state<Item[]>([]);
  let busy = $state(false);
  let failed = $state('');
  let picker = $state<HTMLInputElement | null>(null);
  let next = 0;

  /// The audio every song already holds, to find a file that is already in the library.
  let known: Promise<Map<string, string>> = musicLibrary()
    .then((songs) => {
      const m = new Map<string, string>();
      for (const s of songs)
        if (typeof s.props.audio === 'string') m.set(s.props.audio, s.title ?? '');
      return m;
    })
    .catch(() => new Map());

  async function read(item: Item, file: File) {
    try {
      const asset = await ingestFile(file);
      const at = items.find((i) => i.key === item.key);
      if (!at) return;
      at.asset = asset;
      at.form = formOf(asset, file.name);
      const mime = typeof asset.props.mime === 'string' ? asset.props.mime : '';
      const same = (await known).get(asset.assets[0] ?? '');
      if (!mime.startsWith('audio/')) {
        at.state = 'failed';
        at.why = 'not an audio file';
      } else if (same !== undefined) {
        at.state = 'known';
        at.why = `already in your music${same ? ` as “${same}”` : ''}`;
      } else {
        at.state = 'ready';
      }
    } catch (e) {
      const at = items.find((i) => i.key === item.key);
      if (at) {
        at.state = 'failed';
        at.why = String(e);
      }
    }
  }

  async function choose(files: FileList | File[] | null) {
    const list = Array.from(files ?? []);
    if (!list.length) return;
    const added: { item: Item; file: File }[] = list.map((file) => ({
      file,
      item: {
        key: next++,
        name: file.name,
        state: 'reading',
        form: { ...emptyForm(), ...fromName(file.name) },
        open: false,
      },
    }));
    items = [...items, ...added.map((a) => a.item)];
    // One after another: a phone sends each file in parts, and two at once only race for it.
    for (const a of added) await read(a.item, a.file);
  }
  const emptyForm = (): Form => ({
    title: '',
    artist: '',
    album: '',
    album_artist: '',
    track: '',
    disc: '',
    year: '',
    genre: '',
  });

  const ready = $derived(items.filter((i) => i.state === 'ready' && i.form.title.trim()));
  const reading = $derived(items.some((i) => i.state === 'reading'));
  const single = $derived(items.length === 1);

  const draftOf = (item: Item): NoteDraft => songDraft(item.asset!, item.form);

  async function save() {
    if (busy || !ready.length || reading) return;
    busy = true;
    failed = '';
    try {
      const drafts = ready.map(draftOf);
      const what =
        drafts.length === 1 ? `added “${drafts[0].title}”` : `added ${drafts.length} songs`;
      // A write takes at most 200 notes; a large folder is several.
      for (let at = 0; at < drafts.length; at += 200)
        await createNotesUndoable(drafts.slice(at, at + 200), what);
      onsaved();
      onclose();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  let over = $state(false);
  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    void choose(e.dataTransfer?.files ?? null);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="song-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div
    class="sheet"
    class:over
    role="dialog"
    aria-modal="true"
    aria-label="add a song"
    tabindex="-1"
    ondragover={(e) => {
      e.preventDefault();
      over = true;
    }}
    ondragleave={() => (over = false)}
    ondrop={drop}
  >
    <header>
      <h2>Add a song</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    <input
      bind:this={picker}
      class="file"
      type="file"
      accept="audio/*"
      multiple
      aria-label="choose songs"
      onchange={(e) => {
        void choose((e.currentTarget as HTMLInputElement).files);
        (e.currentTarget as HTMLInputElement).value = '';
      }}
    />
    <button type="button" class="pick" onclick={() => picker?.click()}>
      <Icon name="plus" size={16} />
      {items.length ? 'Choose more files' : 'Choose files'}
    </button>
    {#if !items.length}
      <p class="quiet">Or drop them here. Each song fills itself from what its file says.</p>
      <p class="quiet"><ManualLink page="user/music.html" section="adding-a-song" /></p>
    {/if}

    {#if single}
      {@const item = items[0]}
      <p class="quiet" role="status">
        {item.name}{item.state === 'reading' ? ' — reading…' : ''}{item.why ? ` — ${item.why}` : ''}
      </p>
      {#if item.state === 'ready' || item.state === 'reading'}
        <div class="fields">
          {#each [...TEXT.slice(0, 3), ...NUMBER.slice(0, 1), ...NUMBER.slice(2), 'genre'] as k (k)}
            {@const f = k as keyof Form}
            <label class:wide={f === 'title'}>
              <span>{LABEL[f]}</span>
              <input
                bind:value={item.form[f]}
                inputmode={NUMBER.includes(f) ? 'numeric' : undefined}
                aria-label={LABEL[f]}
              />
            </label>
          {/each}
        </div>
      {/if}
    {:else if items.length}
      <ul class="lines" aria-label="songs to add">
        {#each items as item (item.key)}
          <li class:off={item.state === 'failed' || item.state === 'known'}>
            <button
              type="button"
              class="line"
              disabled={item.state !== 'ready'}
              aria-expanded={item.open}
              onclick={() => (item.open = !item.open)}
            >
              <span class="name">
                <span class="title">{item.form.title || item.name}</span>
                <span class="by">
                  {#if item.state === 'reading'}reading…{:else if item.why}{item.why}{:else}{[
                      item.form.artist,
                      item.form.album,
                    ]
                      .filter(Boolean)
                      .join(' · ') || 'no artist or album'}{/if}
                </span>
              </span>
              {#if item.state === 'ready'}<Icon name="pen" size={14} />{/if}
            </button>
            {#if item.open}
              <div class="fields">
                {#each [...TEXT.slice(0, 3), 'track', 'year', 'genre'] as k (k)}
                  {@const f = k as keyof Form}
                  <label class:wide={f === 'title'}>
                    <span>{LABEL[f]}</span>
                    <input bind:value={item.form[f]} aria-label={`${LABEL[f]} of ${item.name}`} />
                  </label>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    {#if items.length}
      <button type="button" class="go" onclick={save} disabled={busy || reading || !ready.length}>
        {ready.length > 1 ? `Save ${ready.length} songs` : 'Save'}
      </button>
    {/if}
  </div>
</div>

<style>
  .song-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
  }
  .scrim {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .sheet {
    position: relative;
    width: min(38rem, 100%);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom), var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
    animation: sheet-up var(--dur-med) var(--ease);
    outline: none;
  }
  .sheet.over {
    box-shadow:
      var(--shadow-lg),
      inset 0 0 0 2px var(--accent);
  }
  @keyframes sheet-up {
    from {
      transform: translateY(100%);
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  .x {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .file {
    display: none;
  }
  .pick {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: 3rem;
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .quiet,
  .by {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
    gap: var(--space-2);
  }
  .fields .wide {
    grid-column: 1 / -1;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  label input {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: var(--text-base);
    color: var(--text);
  }
  .lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .lines li {
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--space-2);
  }
  .lines li.off .title {
    color: var(--text-muted);
  }
  .line {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 3rem;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .line:disabled {
    cursor: default;
  }
  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .by {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .go {
    min-height: 3rem;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .go:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>

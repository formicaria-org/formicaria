<script lang="ts">
  // **A day's journal entry, ready to write in** (Home, 2026-11-23; the owner: *"you enter and the
  // entry of the day is there, ready to be written"*, as in Logseq). The entry is a note titled with
  // the day (`2026-11-23`) and tagged `journal`, the form imported journals already take. It is made
  // on the **first keystroke**, so no empty day is ever written (Logseq writes one file a day). Saved
  // as you type, a little after you stop.
  import { onMount, untrack } from 'svelte';
  import * as ipc from '../lib/ipc';
  import { formatMoment } from '../lib/appTime';
  import LiveEditor from '../lib/editor/LiveEditor.svelte';

  let {
    day,
    onopen,
    compact = false,
  }: { day: string; onopen: (id: string) => void; compact?: boolean } = $props();

  let note = $state<{ id: string; version: string } | null>(null);
  let draft = $state('');
  let loaded = $state(false);
  let saving = $state<Promise<void> | null>(null);
  let failed = $state('');
  let dirty = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let editor = $state<{ focus: (o?: { preventScroll?: boolean }) => void } | undefined>(undefined);

  /** The day, worded: `Monday, 23 November 2026`. Noon, so no zone moves it to another day. */
  const worded = $derived(
    formatMoment(`${day}T12:00:00Z`, {
      weekday: 'long',
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    }),
  );

  async function load() {
    // Never over what is being typed: the change it would bring is this entry's own save.
    if (dirty || saving) return;
    const [n] = await ipc.journal(day, 1).catch(() => []);
    if (n && n.title === day) {
      const d = await ipc.getNote(n.id).catch(() => null);
      if (d && !dirty) {
        note = { id: d.id, version: d.version };
        draft = d.body;
      }
    } else if (!dirty) {
      note = null;
      draft = '';
    }
    loaded = true;
  }

  async function save() {
    const text = draft;
    try {
      if (note) {
        const version = await ipc.updateBody(note.id, text, note.version);
        note = { id: note.id, version: version || note.version };
      } else if (text.trim()) {
        const [made] = await ipc.createNotes([
          { title: day, body: text, tags: ['journal'], start: '', props: {} },
        ]);
        const d = made ? await ipc.getNote(made.id) : null;
        if (d) note = { id: d.id, version: d.version };
      }
      failed = '';
      if (draft === text) dirty = false;
    } catch (e) {
      failed = `Not saved: ${String(e)}`;
    }
  }

  function typed() {
    dirty = true;
    clearTimeout(timer);
    timer = setTimeout(() => {
      // One save at a time: the first keystroke's makes the note, and the next must write to it.
      const run = async () => {
        if (saving) await saving;
        saving = save();
        await saving;
        saving = null;
      };
      void run();
    }, 600);
  }

  /** Into the entry, at its end: Home pressed on Home, or the journal opened. */
  export function focus() {
    editor?.focus();
  }

  $effect(() => {
    void day;
    untrack(() => {
      dirty = false;
      loaded = false;
      void load();
    });
  });
  onMount(() => {
    const stop = ipc.listenChanges(() => void load());
    return () => {
      stop();
      // A last keystroke inside the wait is still saved when the page moves on.
      if (dirty) {
        clearTimeout(timer);
        void save();
      }
    };
  });
</script>

<section class="entry" class:compact aria-label={`journal, ${day}`}>
  <header>
    <h2>{worded}</h2>
    {#if note}
      <button type="button" class="open" onclick={() => note && onopen(note.id)}
        >Open as a note</button
      >
    {/if}
  </header>
  {#if loaded}
    <div class="writing">
      <LiveEditor
        bind:this={editor}
        bind:value={draft}
        oninput={typed}
        label={`journal for ${day}`}
        undoLabel={`typing in the journal (${day})`}
      />
    </div>
    {#if failed}<p class="error" role="alert">{failed}</p>{/if}
  {/if}
</section>

<style>
  .entry {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  h2 {
    margin: 0;
    font-size: var(--text-md);
  }
  .open {
    margin-left: auto;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    background: none;
    border: none;
    cursor: pointer;
  }
  .writing {
    flex: 1;
    margin-top: var(--space-2);
    cursor: text;
  }
  /* A day's entry is a few lines, not a page: the editor's note-sized height (22rem) and its room
     below the last line on touch (for a whole note's caret) are left to the note's own panel. */
  .writing :global(.editor) {
    min-height: 9rem;
  }
  .compact .writing :global(.editor) {
    min-height: 5rem;
  }
  .writing :global(.cm-content) {
    padding-bottom: var(--space-3) !important;
  }
  .error {
    color: var(--danger-fg);
    margin: var(--space-1) 0 0;
  }
</style>

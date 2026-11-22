<script lang="ts">
  // **Home** (the shell, P2b; `unifying-home-research-2026-11-22.md` §4 and §10.2; the owner: *"the
  // initial home screen must be customizable"*). What you can do here (the library's sections, each
  // with what it holds and ＋), then blocks: getting started, today, just in, recent, pinned, and any
  // view or note the person adds. **Kept as a note** tagged `home`, one block per line
  // (`![Today](view:today)`, `![Menu](view:Menu)`, `![](note:<id>)`), so it syncs and has history
  // like any note. *Customize home* adds, moves and removes blocks and sets one or two columns;
  // *Reset* goes back to the default.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import sectionsJson from '../lib/sections.json';
  import type { ObjectMeta, Section, SectionSummary, ViewInfo } from '../lib/types';
  import { appToday, dayOfInstant } from '../lib/appTime';
  import Icon from '../lib/Icon.svelte';

  let { go, onopen }: { go: (url: string) => void; onopen: (id: string) => void } = $props();

  const sections = sectionsJson as Section[];
  type Block = { kind: string; label: string };
  const BUILT: Record<string, string> = {
    start: 'Getting started',
    today: 'Today',
    justin: 'Just in',
    recent: 'Recent',
    pinned: 'Pinned',
  };
  const DEFAULT: Block[] = ['start', 'today', 'justin', 'recent', 'pinned'].map((k) => ({
    kind: k,
    label: BUILT[k],
  }));

  let summary = $state<SectionSummary[]>([]);
  let recent = $state<ObjectMeta[]>([]);
  let agenda = $state<ObjectMeta[]>([]);
  let pinned = $state<ObjectMeta[]>([]);
  let views = $state<ViewInfo[]>([]);
  let viewRows = $state<Record<string, ObjectMeta[]>>({});
  let embedded = $state<Record<string, ObjectMeta | null>>({});
  let home = $state<{ id: string; version: string } | null>(null);
  let blocks = $state<Block[]>(DEFAULT);
  let columns = $state<1 | 2>(2);
  let paired = $state(false);
  let editing = $state(false);
  let saveError = $state('');

  /** `![Label](view:x)` / `![](note:id)` lines → blocks. Anything else is left out. */
  function parse(body: string): Block[] {
    const out: Block[] = [];
    for (const m of body.matchAll(/!\[([^\]]*)\]\(((?:view|note):[^)]+)\)/g)) {
      out.push({ kind: m[2], label: m[1] });
    }
    return out.map((b) =>
      b.kind.startsWith('view:') && BUILT[b.kind.slice(5)]
        ? { kind: b.kind.slice(5), label: b.label || BUILT[b.kind.slice(5)] }
        : b,
    );
  }
  function write(list: Block[]): string {
    return (
      list
        .map((b) => (BUILT[b.kind] ? `![${b.label}](view:${b.kind})` : `![${b.label}](${b.kind})`))
        .join('\n') + '\n'
    );
  }

  async function read() {
    const [s, r, a, p, v, h, alive] = await Promise.all([
      ipc.sectionSummary().catch(() => []),
      ipc.recent().catch(() => []),
      ipc.getAgenda().catch(() => []),
      ipc.tagged('pinned').catch(() => []),
      ipc.listViews().catch(() => []),
      ipc.tagged('home', 1).catch(() => []),
      ipc.alive().catch(() => null),
    ]);
    summary = s;
    recent = r;
    agenda = a;
    pinned = p;
    views = v.filter((x) => !x.error);
    paired = !!alive && 'devices' in alive && (alive.devices ?? 0) > 0;
    if (h[0]) {
      const d = await ipc.getNote(h[0].id).catch(() => null);
      if (d) {
        home = { id: d.id, version: d.version };
        const parsed = parse(d.body);
        if (parsed.length) blocks = parsed;
        columns = Number(d.props?.columns) === 1 ? 1 : 2;
      }
    }
    await fillBlocks();
  }
  async function fillBlocks() {
    for (const b of blocks) {
      if (b.kind.startsWith('view:') && !(b.kind in viewRows)) {
        const name = b.kind.slice(5);
        const res = await ipc.runView(name).catch(() => null);
        const rows = res?.rows ?? res?.board?.columns.flatMap((c) => c.cards) ?? [];
        viewRows = { ...viewRows, [b.kind]: rows.slice(0, 6) };
      }
      if (b.kind.startsWith('note:') && !(b.kind in embedded)) {
        const n = await ipc.getNote(b.kind.slice(5)).catch(() => null);
        embedded = {
          ...embedded,
          [b.kind]: n ? ({ ...n, preview: n.body.slice(0, 160) } as unknown as ObjectMeta) : null,
        };
      }
    }
  }
  onMount(() => {
    void read();
    return ipc.listenChanges(() => void read());
  });

  async function save(list: Block[], cols = columns) {
    saveError = '';
    const body = write(list);
    try {
      if (home) {
        await ipc.updateBody(home.id, body, home.version);
        await ipc.setProperty(home.id, 'columns', String(cols));
        const d = await ipc.getNote(home.id);
        home = d ? { id: d.id, version: d.version } : home;
      } else {
        const [made] = await ipc.createNotes([
          { title: 'Home', body, tags: ['home'], start: '', props: { columns: cols } },
        ]);
        const d = made ? await ipc.getNote(made.id) : null;
        if (d) home = { id: d.id, version: d.version };
      }
      blocks = list;
      columns = cols;
      await fillBlocks();
    } catch (e) {
      saveError = String(e);
    }
  }
  const move = (i: number, by: number) => {
    const j = i + by;
    if (j < 0 || j >= blocks.length) return;
    const list = [...blocks];
    [list[i], list[j]] = [list[j], list[i]];
    void save(list);
  };
  const remove = (i: number) => void save(blocks.filter((_, k) => k !== i));
  let adding = $state('');
  function add() {
    if (!adding) return;
    const label =
      BUILT[adding] ??
      (adding.startsWith('view:')
        ? adding.slice(5)
        : (recent.find((n) => `note:${n.id}` === adding)?.title ?? ''));
    void save([...blocks, { kind: adding, label }]);
    adding = '';
  }
  let dragFrom = -1;
  function drop(to: number) {
    if (dragFrom < 0 || dragFrom === to) return;
    const list = [...blocks];
    const [b] = list.splice(dragFrom, 1);
    list.splice(to, 0, b);
    dragFrom = -1;
    void save(list);
  }

  const count = (key: string) => summary.find((s) => s.key === key)?.count ?? 0;
  const latest = (key: string) => summary.find((s) => s.key === key)?.latest ?? null;
  const today = $derived(appToday());
  const todayItems = $derived(
    agenda.filter((n) => dayOfInstant(n.start ?? n.due ?? '') === today || (n.due ?? '') <= today),
  );
  const steps = $derived([
    {
      label: 'Write a note',
      done: recent.some((n) => !n.tags.includes('tutorial')),
      go: () => go('/f/group/Library'),
    },
    {
      label: 'Keep a web page',
      done: recent.some((n) => n.kind_key === 'web'),
      go: () => go('/f/new'),
    },
    {
      label: 'Log a meal or a workout',
      done: count('food') + count('fitness') > 0,
      go: () => go('/f/group/Food'),
    },
    {
      label: 'Add a song or a video',
      done: count('music') + count('videos') > 0,
      go: () => go('/f/group/Music'),
    },
    {
      label: 'Connect your phone',
      done: paired,
      go: () => go('/f/settings'),
    },
  ]);
  const justIn = $derived(
    recent
      .filter((n) => ['web', 'song', 'video', 'meeting', 'paper'].includes(n.kind_key ?? ''))
      .slice(0, 6),
  );

  async function newIn(s: Section) {
    const props = (s.new?.props ?? {}) as Record<string, string | number>;
    const [made] = await ipc.createNotes([{ title: '', body: '', tags: [], start: '', props }]);
    if (made) onopen(made.id);
  }
</script>

<div class="home">
  <header class="top">
    <h1>Home</h1>
    <button
      type="button"
      class="customize"
      aria-pressed={editing}
      onclick={() => (editing = !editing)}
    >
      {editing ? 'Done' : 'Customize home'}
    </button>
  </header>

  <h2 class="what">What you can do here</h2>
  <ul class="kinds" aria-label="what you can do here">
    {#each sections as s (s.key)}
      {@const n = latest(s.key)}
      <li class="kind">
        <button type="button" class="open-kind" onclick={() => go(s.page)}>
          <span class="k"><span class="mark" aria-hidden="true">{s.mark}</span> {s.label}</span>
          <span class="n">{count(s.key)}{n ? ` · ${n.title || n.preview}` : ''}</span>
          <span class="purpose">{s.purpose}</span>
        </button>
        <button
          type="button"
          class="plus"
          aria-label={`new in ${s.label}`}
          title={`New in ${s.label}`}
          onclick={() => void newIn(s)}><Icon name="plus" size={14} /></button
        >
      </li>
    {/each}
  </ul>

  {#if editing}
    <div class="edit" role="group" aria-label="customize home">
      <label
        >Add a block
        <select bind:value={adding} aria-label="block to add">
          <option value="">Choose…</option>
          {#each Object.entries(BUILT) as [k, l] (k)}<option value={k}>{l}</option>{/each}
          <optgroup label="A view">
            {#each views as v (v.name)}<option value={`view:${v.name}`}>{v.name}</option>{/each}
          </optgroup>
          <optgroup label="A note">
            {#each recent.slice(0, 15) as n (n.id)}<option value={`note:${n.id}`}
                >{n.title || n.preview}</option
              >{/each}
          </optgroup>
        </select>
      </label>
      <button type="button" onclick={add} disabled={!adding}>Add</button>
      <span class="sep"></span>
      <label class="cols"
        ><input
          type="radio"
          name="home-cols"
          checked={columns === 1}
          onchange={() => void save(blocks, 1)}
        /> One column</label
      >
      <label class="cols"
        ><input
          type="radio"
          name="home-cols"
          checked={columns === 2}
          onchange={() => void save(blocks, 2)}
        /> Two columns</label
      >
      <button type="button" class="reset" onclick={() => void save(DEFAULT, 2)}
        >Reset to the default</button
      >
    </div>
    {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
  {/if}

  <ol class="blocks" class:two={columns === 2} aria-label="home blocks">
    {#each blocks as b, i (b.kind + i)}
      <li
        class="block"
        draggable={editing}
        ondragstart={() => (dragFrom = i)}
        ondragover={(e) => editing && e.preventDefault()}
        ondrop={() => drop(i)}
      >
        <h3>
          {b.label || BUILT[b.kind] || b.kind}
          {#if editing}
            <span class="tools">
              <button
                type="button"
                aria-label={`move ${b.label} up`}
                onclick={() => move(i, -1)}
                disabled={i === 0}>↑</button
              >
              <button
                type="button"
                aria-label={`move ${b.label} down`}
                onclick={() => move(i, 1)}
                disabled={i === blocks.length - 1}>↓</button
              >
              <button type="button" aria-label={`remove ${b.label}`} onclick={() => remove(i)}
                >✕</button
              >
            </span>
          {/if}
        </h3>
        {#if b.kind === 'start'}
          <ul class="steps">
            {#each steps as st (st.label)}
              <li class:done={st.done}>
                <span aria-hidden="true">{st.done ? '✓' : '○'}</span>
                <button type="button" class="link" onclick={st.go}>{st.label}</button>
              </li>
            {/each}
          </ul>
        {:else if b.kind === 'today'}
          {#if todayItems.length}
            <ul class="mini">
              {#each todayItems.slice(0, 8) as n (n.id)}
                <li>
                  <button type="button" class="link" onclick={() => onopen(n.id)}
                    >{n.title || n.preview}</button
                  >
                </li>
              {/each}
            </ul>
          {:else}<p class="muted">Nothing planned or due today.</p>{/if}
          <button type="button" class="more" onclick={() => go('/f/today')}>Open Today</button>
        {:else if b.kind === 'justin'}
          {#if justIn.length}
            <ul class="mini">
              {#each justIn as n (n.id)}
                <li>
                  <button type="button" class="link" onclick={() => onopen(n.id)}
                    >{n.title || n.preview}</button
                  >
                </li>
              {/each}
            </ul>
          {:else}<p class="muted">Pages, songs and meetings you bring in show here.</p>{/if}
        {:else if b.kind === 'recent'}
          <ul class="mini">
            {#each recent.slice(0, 6) as n (n.id)}
              <li>
                <button type="button" class="link" onclick={() => onopen(n.id)}
                  >{n.title || n.preview}</button
                >
              </li>
            {/each}
          </ul>
        {:else if b.kind === 'pinned'}
          {#if pinned.length}
            <ul class="mini">
              {#each pinned as n (n.id)}
                <li>
                  <button type="button" class="link" onclick={() => onopen(n.id)}
                    >{n.title || n.preview}</button
                  >
                </li>
              {/each}
            </ul>
          {:else}<p class="muted">Tag a note <code>pinned</code> to keep it here.</p>{/if}
        {:else if b.kind.startsWith('view:')}
          {#if viewRows[b.kind]?.length}
            <ul class="mini">
              {#each viewRows[b.kind] as n (n.id)}
                <li>
                  <button type="button" class="link" onclick={() => onopen(n.id)}
                    >{n.title || n.preview}</button
                  >
                </li>
              {/each}
            </ul>
          {/if}
          <button
            type="button"
            class="more"
            onclick={() => go(`/f/view/${encodeURIComponent(b.kind.slice(5))}`)}
            >Open {b.kind.slice(5)}</button
          >
        {:else if b.kind.startsWith('note:')}
          {@const n = embedded[b.kind]}
          {#if n}
            <p class="preview">{n.preview}</p>
            <button type="button" class="more" onclick={() => onopen(n.id)}>Open the note</button>
          {:else}<p class="muted">This note is not here.</p>{/if}
        {/if}
      </li>
    {/each}
  </ol>
</div>

<style>
  .home {
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
  }
  .customize {
    margin-left: auto;
    font: inherit;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .what {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .kinds {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
    gap: var(--space-2);
  }
  .kind {
    position: relative;
  }
  .open-kind {
    width: 100%;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    padding-right: 2.4rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .open-kind:hover {
    border-color: var(--accent);
  }
  .k {
    font-weight: 650;
  }
  .n,
  .purpose {
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .purpose {
    white-space: normal;
  }
  .plus {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    width: 1.8rem;
    height: 1.8rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    cursor: pointer;
  }
  .edit {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border: 1px dashed var(--accent);
    border-radius: var(--radius-md);
  }
  .edit .sep {
    flex: 1;
  }
  .blocks {
    list-style: none;
    margin: var(--space-4) 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-3);
  }
  .blocks.two {
    grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
  }
  .block {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
  }
  .block h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .tools {
    margin-left: auto;
    display: inline-flex;
    gap: var(--space-1);
  }
  .tools button {
    border: 1px solid var(--border);
    background: var(--bg);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .steps,
  .mini {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .steps li {
    display: flex;
    gap: var(--space-2);
    padding: 2px 0;
  }
  .steps li.done .link {
    color: var(--text-muted);
    text-decoration: line-through;
  }
  .mini li {
    padding: 2px 0;
    border-bottom: 1px dashed var(--border);
  }
  .link {
    font: inherit;
    color: var(--text);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .link:hover {
    text-decoration: underline;
  }
  .more {
    margin-top: var(--space-2);
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .muted {
    color: var(--text-muted);
    margin: 0;
  }
  .preview {
    margin: 0;
    white-space: pre-line;
  }
  .error {
    color: var(--danger-fg);
  }
</style>

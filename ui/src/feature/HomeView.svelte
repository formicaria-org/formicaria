<script lang="ts">
  // **Home: a glance, a journal, cards** (`unifying-home-research-2026-11-22.md` §12, 2026-11-23).
  // The owner: Home should give *"a quick glance on the current status, and the user selects what
  // to see"*; a board as its figures and what needs a look, or the whole board, beside anything
  // else chosen; a journal entry for the day, ready to write in, as in Logseq; calm on a small
  // screen, with quick links to what is kept an eye on.
  //
  // - **The glance row**: figures (a section this week, what is due, a board's open cards).
  // - **The watch row**: links to notes, views and sections kept an eye on.
  // - **The journal**: today's entry, ready to write in.
  // - **Cards** at their size: summary (what needs a look) or full (the whole thing).
  //
  // What is on Home and how is kept in the Home note (`lib/home.ts`); with nothing set up, Home is
  // generated from what the person uses. **Edit Home** lists the cards (size, on the phone, order)
  // and what can be added; **Pin to Home** works from a note's ⋯ and a section's bar.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import sectionsJson from '../lib/sections.json';
  import type { ObjectMeta, Section, SectionSummary, ViewInfo } from '../lib/types';
  import { appToday, dayOfInstant } from '../lib/appTime';
  import { addDays } from '../lib/civil';
  import {
    BUILTIN,
    generated,
    labelOf,
    layout,
    readHome,
    saveHome,
    sizesOf,
    type Card,
    type CardSize,
    type HomeDoc,
  } from '../lib/home';
  import { webTarget } from '../lib/features';
  import { searchUrl } from '../lib/privacy.svelte';
  import JournalEntry from './JournalEntry.svelte';

  let { go, onopen }: { go: (url: string) => void; onopen: (id: string) => void } = $props();

  const sections = sectionsJson as Section[];
  const sectionOf = (key: string) => sections.find((s) => s.key === key);

  let summary = $state<SectionSummary[]>([]);
  let recent = $state<ObjectMeta[]>([]);
  let agenda = $state<ObjectMeta[]>([]);
  let pinned = $state<ObjectMeta[]>([]);
  let views = $state<ViewInfo[]>([]);
  let paired = $state(false);
  /** What each card that reads for itself has read, by its ref. */
  let rows = $state<Record<string, ObjectMeta[]>>({});
  let notes = $state<Record<string, { id: string; title: string; body: string } | null>>({});
  let home = $state<{ id: string; version: string } | null>(null);
  let doc = $state<HomeDoc>({ cards: [], text: '' });
  let editing = $state(false);
  let saveError = $state('');
  let narrow = $state(false);
  let root = $state<HTMLDivElement | undefined>(undefined);
  let entry = $state<{ focus: () => void } | undefined>(undefined);

  /** The cards shown: the person's, or the generated ones while nothing is set up. */
  const cards = $derived(doc.cards.length ? doc.cards : generated(summary));
  const set = $derived(doc.cards.length > 0);
  const placed = $derived(layout(cards, narrow));
  const today = $derived(appToday());

  async function read() {
    const [s, r, a, p, v, h, alive] = await Promise.all([
      ipc.sectionSummary().catch(() => []),
      ipc.recent().catch(() => []),
      ipc.getAgenda().catch(() => []),
      ipc.tagged('pinned').catch(() => []),
      ipc.listViews().catch(() => []),
      readHome(),
      ipc.alive().catch(() => null),
    ]);
    summary = s;
    recent = r;
    agenda = a;
    pinned = p;
    views = v.filter((x) => !x.error);
    paired = !!alive && 'devices' in alive && (alive.devices ?? 0) > 0;
    home = h ? { id: h.id, version: h.version } : null;
    doc = h?.doc ?? { cards: [], text: '' };
    await fill();
  }
  /** Each card that shows notes of its own reads them: a board, a section, a view, a note. */
  async function fill() {
    const next: Record<string, ObjectMeta[]> = {};
    const nextNotes: typeof notes = {};
    await Promise.all(
      cards.map(async (c) => {
        const [fam, arg] = [c.ref.split(':')[0], c.ref.slice(c.ref.indexOf(':') + 1)];
        if (fam === 'board') next[c.ref] = await ipc.sectionNotes(arg, 500).catch(() => []);
        else if (fam === 'section') next[c.ref] = await ipc.sectionNotes(arg, 20).catch(() => []);
        else if (fam === 'view') {
          const res = await ipc.runView(arg).catch(() => null);
          next[c.ref] = res?.rows ?? res?.board?.columns.flatMap((col) => col.cards) ?? [];
        } else if (fam === 'note') {
          const n = await ipc.getNote(arg).catch(() => null);
          nextNotes[c.ref] = n ? { id: n.id, title: n.title ?? '', body: n.body } : null;
        }
      }),
    );
    rows = next;
    notes = nextNotes;
  }

  onMount(() => {
    void read();
    const stop = ipc.listenChanges(() => void read());
    const again = () => {
      if (placed.journal) entry?.focus();
      else go('/f/today');
    };
    addEventListener('fm-home-again', again);
    // Narrow by Home's own width (a phone, or a pane on a tablet), not the window's.
    const measure = () => (narrow = !!root && root.clientWidth > 0 && root.clientWidth < 700);
    const ro = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    if (root) ro?.observe(root);
    measure();
    return () => {
      stop();
      removeEventListener('fm-home-again', again);
      ro?.disconnect();
    };
  });

  async function save(next: HomeDoc) {
    saveError = '';
    try {
      home = await saveHome(next, home);
      doc = next;
      await fill();
    } catch (e) {
      saveError = String(e);
    }
  }
  /** The cards as they stand, made the person's own on the first edit. */
  const own = (): Card[] =>
    (doc.cards.length ? doc.cards : generated(summary)).map((c) => ({ ...c }));
  const change = (i: number, patch: Partial<Card>) => {
    const list = own();
    list[i] = { ...list[i], ...patch };
    void save({ ...doc, cards: list });
  };
  const move = (i: number, by: number) => {
    const list = own();
    const j = i + by;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    void save({ ...doc, cards: list });
  };
  const remove = (i: number) => void save({ ...doc, cards: own().filter((_, k) => k !== i) });
  function add(ref: string, size?: CardSize) {
    if (!ref || cards.some((c) => c.ref === ref)) return;
    const sizes = sizesOf(ref);
    const card: Card = {
      ref,
      label: '',
      size: size && sizes.includes(size) ? size : sizes.includes('summary') ? 'summary' : sizes[0],
      phone: BUILTIN[ref]?.phone ?? false,
    };
    void save({ ...doc, cards: [...own(), card] });
  }
  /** Reset: nothing set up, so Home is generated from use again. */
  const reset = () => void save({ cards: [], text: doc.text });
  let words = $state('');
  $effect(() => {
    if (editing) words = doc.text;
  });
  let dragFrom = -1;
  function drop(to: number) {
    if (dragFrom < 0 || dragFrom === to) return;
    const list = own();
    const [c] = list.splice(dragFrom, 1);
    list.splice(to, 0, c);
    dragFrom = -1;
    void save({ ...doc, cards: list });
  }

  // **What can be added**, as groups of choices (a list of toggles, never a blank canvas).
  const catalogue = $derived([
    {
      group: 'Today',
      items: Object.keys(BUILTIN).map((k) => ({ ref: k, label: BUILTIN[k].label })),
    },
    {
      group: 'Figures',
      items: sections
        .filter((s) => s.key !== 'discussions' && s.key !== 'collaboration')
        .map((s) => ({ ref: `stat:${s.key}`, label: `${s.label} this week`, size: 'glance' })),
    },
    {
      group: 'Boards',
      items: [{ ref: 'board:tasks', label: 'Tasks board' }],
    },
    {
      group: 'Sections',
      items: sections.map((s) => ({ ref: `section:${s.key}`, label: s.label })),
    },
    { group: 'Views', items: views.map((v) => ({ ref: `view:${v.name}`, label: v.name })) },
    {
      group: 'Notes',
      items: recent
        .filter((n) => !n.tags.includes('home') && !n.tags.includes('journal'))
        .slice(0, 15)
        .map((n) => ({ ref: `note:${n.id}`, label: n.title || n.preview })),
    },
  ] as { group: string; items: { ref: string; label: string; size?: CardSize }[] }[]);
  let adding = $state('');

  // ---- what the cards show ----
  const count = (key: string) => summary.find((s) => s.key === key)?.count ?? 0;
  /** `YYYY-MM-DD`, `n` days before today, by the calendar (`civil.ts`). */
  const back = (n: number): string => addDays(today, -n);
  /** Notes made per day in a section, over the last `days` days, oldest first. */
  function perDay(key: string, days: number): number[] {
    const made = (summary.find((s) => s.key === key)?.recent ?? []).map((t) => dayOfInstant(t));
    return Array.from(
      { length: days },
      (_, i) => made.filter((d) => d === back(days - 1 - i)).length,
    );
  }
  const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
  function stat(key: string) {
    const fortnight = perDay(key, 14);
    const week = sum(fortnight.slice(7));
    const before = sum(fortnight.slice(0, 7));
    return { week, trend: week > before ? '↑' : week < before ? '↓' : '' };
  }
  const due = $derived(
    agenda.filter((n) => dayOfInstant(n.start ?? '') === today || (!!n.due && n.due <= today)),
  );
  const overdueDue = $derived(due.filter((n) => !!n.due && n.due < today));
  const justIn = $derived(
    recent.filter(
      (n) =>
        ['web', 'song', 'video', 'meeting', 'paper'].includes(n.kind_key ?? '') &&
        dayOfInstant(n.created) >= back(6),
    ),
  );
  const carryOn = $derived(
    recent.find(
      (n) => !n.tags.some((t) => ['home', 'journal', 'tutorial'].includes(t)) && n.title !== today,
    ) ?? null,
  );
  /** A board's state: its columns, and what needs a look (overdue, blocked, unmoved a week). */
  function board(ref: string) {
    const all = rows[ref] ?? [];
    const order = ['todo', 'doing', 'blocked', 'done'];
    const statuses = [...new Set(all.map((n) => n.status || 'none'))].sort(
      (a, b) => (order.indexOf(a) + 1 || 99) - (order.indexOf(b) + 1 || 99),
    );
    const open = all.filter((n) => n.status !== 'done');
    const week = back(7);
    const look = open
      .map((n) => ({
        n,
        why:
          n.due && n.due < today
            ? 'overdue'
            : n.status === 'blocked'
              ? 'blocked'
              : dayOfInstant(n.updated) < week
                ? 'unmoved a week'
                : '',
      }))
      .filter((x) => x.why);
    return {
      columns: statuses.map((s) => ({
        status: s,
        cards: all.filter((n) => (n.status || 'none') === s),
      })),
      open: open.length,
      overdue: look.filter((x) => x.why === 'overdue').length,
      look,
    };
  }
  const steps = $derived([
    {
      label: 'Write in today’s journal',
      done: recent.some((n) => n.tags.includes('journal')),
      go: () => go('/f/today'),
    },
    { label: 'Keep a web page', done: count('web') > 0, go: () => go('/f/group/Web') },
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
    { label: 'Connect your phone', done: paired, go: () => go('/f/settings') },
  ]);

  /** Where a card's title goes: its whole place. */
  function whole(c: Card): (() => void) | null {
    const [fam, arg] = [c.ref.split(':')[0], c.ref.slice(c.ref.indexOf(':') + 1)];
    if (fam === 'stat' || fam === 'board' || fam === 'section') {
      const s = sectionOf(arg);
      return s ? () => go(s.page) : null;
    }
    if (fam === 'view') return () => go(`/f/view/${encodeURIComponent(arg)}`);
    if (fam === 'note') return () => onopen(arg);
    if (c.ref === 'due') return () => go('/f/agenda');
    if (c.ref === 'journal') return () => go('/f/today');
    if (c.ref === 'justin' || c.ref === 'recent') return () => go('/f/timeline');
    return null;
  }
  const title = (n: ObjectMeta) => n.title || n.preview || 'Untitled';

  // ---- the capture bar (a narrow screen): a line into today's entry, or a page to open ----
  let captured = $state('');
  let captureNote = $state('');
  async function capture() {
    const t = captured.trim();
    if (!t) return;
    const web = webTarget(t, searchUrl);
    if (web && !/\s/.test(t) && !web.label.startsWith('Search')) {
      captured = '';
      go(web.url);
      return;
    }
    try {
      const [n] = await ipc.journal(today, 1);
      if (n && n.title === today) {
        const d = await ipc.getNote(n.id);
        if (!d) throw new Error('today’s entry could not be read');
        const body = d.body.trimEnd();
        await ipc.updateBody(d.id, `${body}${body ? '\n' : ''}- ${t}\n`, d.version);
      } else {
        await ipc.createNotes([
          { title: today, body: `- ${t}\n`, tags: ['journal'], start: '', props: {} },
        ]);
      }
      captured = '';
      captureNote = 'Added to today’s journal.';
    } catch (e) {
      captureNote = `Not added: ${String(e)}`;
    }
  }
</script>

<div class="home" class:narrow bind:this={root}>
  <header class="top">
    <h1>Home</h1>
    <button
      type="button"
      class="customize"
      aria-pressed={editing}
      onclick={() => (editing = !editing)}
    >
      {editing ? 'Done' : 'Edit Home'}
    </button>
  </header>

  {#if editing}
    <section class="edit" aria-label="edit home">
      <p class="note">
        {set
          ? 'Your Home. Each card: its size, whether it is on the phone, its place.'
          : 'Home is made from what you use until you change it. Changing anything here makes it yours.'}
      </p>
      <ol class="list" aria-label="home cards">
        {#each cards as c, i (c.ref)}
          {@const name = labelOf(c, sections)}
          <li
            draggable="true"
            ondragstart={() => (dragFrom = i)}
            ondragover={(e) => e.preventDefault()}
            ondrop={() => drop(i)}
          >
            <span class="name">{name}</span>
            <select
              aria-label={`size of ${name}`}
              value={c.size}
              onchange={(e) =>
                change(i, { size: (e.currentTarget as HTMLSelectElement).value as CardSize })}
            >
              {#each sizesOf(c.ref) as z (z)}<option value={z}
                  >{z === 'glance' ? 'Glance' : z === 'summary' ? 'Summary' : 'Full'}</option
                >{/each}
            </select>
            <label class="phone"
              ><input
                type="checkbox"
                checked={c.phone}
                aria-label={`${name} on the phone`}
                onchange={(e) =>
                  change(i, { phone: (e.currentTarget as HTMLInputElement).checked })}
              /> On phone</label
            >
            <button
              type="button"
              aria-label={`move ${name} up`}
              onclick={() => move(i, -1)}
              disabled={i === 0}>↑</button
            >
            <button
              type="button"
              aria-label={`move ${name} down`}
              onclick={() => move(i, 1)}
              disabled={i === cards.length - 1}>↓</button
            >
            <button type="button" aria-label={`remove ${name}`} onclick={() => remove(i)}>✕</button>
          </li>
        {/each}
      </ol>
      <div class="add">
        <label
          >Add
          <select bind:value={adding} aria-label="card to add">
            <option value="">Choose…</option>
            {#each catalogue as g (g.group)}
              {#if g.items.length}
                <optgroup label={g.group}>
                  {#each g.items as it (it.ref)}
                    <option value={it.ref} disabled={cards.some((c) => c.ref === it.ref)}
                      >{it.label}</option
                    >
                  {/each}
                </optgroup>
              {/if}
            {/each}
          </select>
        </label>
        <button
          type="button"
          disabled={!adding}
          onclick={() => {
            const it = catalogue.flatMap((g) => g.items).find((x) => x.ref === adding);
            add(adding, it?.size);
            adding = '';
          }}>Add</button
        >
        <button type="button" class="reset" onclick={reset} disabled={!set}
          >Reset to the default</button
        >
      </div>
      <label class="words"
        >Your own words at the top of Home
        <textarea
          rows="3"
          bind:value={words}
          onblur={() => words !== doc.text && void save({ cards: own(), text: words })}></textarea>
      </label>
      {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
    </section>
  {/if}

  {#if placed.glance.length}
    <ul class="glance" aria-label="at a glance">
      {#each placed.glance as c (c.ref)}
        {@const fam = c.ref.split(':')[0]}
        {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
        <li>
          <button type="button" onclick={() => whole(c)?.()}>
            {#if fam === 'stat'}
              {@const st = stat(arg)}
              <b>{st.week}{st.trend ? ` ${st.trend}` : ''}</b><span>{labelOf(c, sections)}</span>
            {:else if fam === 'board'}
              {@const b = board(c.ref)}
              <b>{b.open}</b><span
                >open in {sectionOf(arg)?.label ?? arg}{b.overdue
                  ? ` · ${b.overdue} overdue`
                  : ''}</span
              >
            {:else if c.ref === 'due'}
              <b>{due.length}</b><span
                >due today{overdueDue.length ? ` · ${overdueDue.length} overdue` : ''}</span
              >
            {:else if c.ref === 'justin'}
              <b>{justIn.length}</b><span>came in this week</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if placed.watch.length}
    <ul class="watch" aria-label="kept an eye on">
      {#each placed.watch as c (c.ref)}
        {@const fam = c.ref.split(':')[0]}
        {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
        <li>
          <button type="button" onclick={() => whole(c)?.()}>
            {#if fam === 'note'}{c.label || notes[c.ref]?.title || 'A note'}
            {:else if fam === 'section'}{labelOf(c, sections)} <span class="n">{count(arg)}</span>
            {:else}{labelOf(c, sections)}{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if doc.text}<p class="words-shown">{doc.text}</p>{/if}

  <div class="main" class:with-journal={!!placed.journal}>
    {#if placed.journal}
      <div class="journal">
        <JournalEntry bind:this={entry} day={today} {onopen} compact={narrow} />
        {#if placed.journal.size === 'full'}
          <button type="button" class="more" onclick={() => go('/f/today')}>Earlier days</button>
        {/if}
      </div>
    {/if}
    <ol class="cards" aria-label="home cards shown">
      {#each placed.main as c (c.ref)}
        {@const fam = c.ref.split(':')[0]}
        {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
        {@const full = c.size === 'full'}
        {@const open = whole(c)}
        <li class="card" class:full>
          <h3>
            {#if open}<button type="button" class="title" onclick={open}
                >{labelOf(c, sections)}</button
              >
            {:else}{labelOf(c, sections)}{/if}
          </h3>
          {#if c.ref === 'due'}
            {#if due.length}
              <ul class="mini">
                {#each due.slice(0, full ? 30 : 6) as n (n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(n.id)}
                      >{title(n)}</button
                    >
                    {#if n.due && n.due < today}<span class="why">overdue</span>{/if}
                  </li>
                {/each}
              </ul>
            {:else}<p class="muted">Nothing planned or due today.</p>{/if}
          {:else if c.ref === 'justin'}
            {#if justIn.length}
              <ul class="mini">
                {#each justIn.slice(0, full ? 20 : 6) as n (n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(n.id)}
                      >{title(n)}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}<p class="muted">Pages, songs and meetings you bring in show here.</p>{/if}
          {:else if c.ref === 'continue'}
            {#if carryOn}
              <button
                type="button"
                class="link strong"
                onclick={() => carryOn && onopen(carryOn.id)}>{title(carryOn)}</button
              >
              {#if carryOn.preview && carryOn.preview !== carryOn.title}<p class="preview">
                  {carryOn.preview}
                </p>{/if}
            {:else}<p class="muted">The note you were in last shows here.</p>{/if}
          {:else if c.ref === 'recent'}
            <ul class="mini">
              {#each recent.slice(0, full ? 15 : 6) as n (n.id)}
                <li>
                  <button type="button" class="link" onclick={() => onopen(n.id)}>{title(n)}</button
                  >
                </li>
              {/each}
            </ul>
          {:else if c.ref === 'pinned'}
            {#if pinned.length}
              <ul class="mini">
                {#each pinned as n (n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(n.id)}
                      >{title(n)}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}<p class="muted">Tag a note <code>pinned</code> to keep it here.</p>{/if}
          {:else if c.ref === 'start'}
            <ul class="steps">
              {#each steps as st (st.label)}
                <li class:done={st.done}>
                  <span aria-hidden="true">{st.done ? '✓' : '○'}</span>
                  <button type="button" class="link" onclick={st.go}>{st.label}</button>
                </li>
              {/each}
            </ul>
          {:else if fam === 'stat'}
            {@const days = perDay(arg, full ? 30 : 7)}
            {@const top = Math.max(1, ...days)}
            <p class="figure"><b>{sum(days)}</b> in the last {full ? 30 : 7} days</p>
            <div class="bars" aria-hidden="true">
              {#each days as d, i (i)}<span
                  style={`height:${Math.round((d / top) * 100)}%`}
                  title={`${d}`}
                ></span>{/each}
            </div>
          {:else if fam === 'board'}
            {@const b = board(c.ref)}
            <p class="figure">
              {#each b.columns.filter((col) => col.status !== 'done') as col, k (col.status)}{k
                  ? ' · '
                  : ''}{col.cards.length}
                {col.status}{/each}
            </p>
            {#if full}
              <div class="columns">
                {#each b.columns as col (col.status)}
                  <div class="column">
                    <h4>{col.status} <span class="n">{col.cards.length}</span></h4>
                    {#each col.cards.slice(0, 10) as n (n.id)}
                      <button type="button" class="chip-card" onclick={() => onopen(n.id)}
                        >{title(n)}</button
                      >
                    {/each}
                  </div>
                {/each}
              </div>
            {:else if b.look.length}
              <ul class="mini">
                {#each b.look.slice(0, 6) as x (x.n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(x.n.id)}
                      >{title(x.n)}</button
                    >
                    <span class="why">{x.why}</span>
                  </li>
                {/each}
              </ul>
            {:else}<p class="muted">Nothing overdue, blocked or stuck.</p>{/if}
          {:else if fam === 'section' || fam === 'view'}
            {#if rows[c.ref]?.length}
              <ul class="mini">
                {#each rows[c.ref].slice(0, full ? 20 : 6) as n (n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(n.id)}
                      >{title(n)}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}<p class="muted">Nothing here yet.</p>{/if}
          {:else if fam === 'note'}
            {@const n = notes[c.ref]}
            {#if n}
              <p class="preview">{full ? n.body : n.body.slice(0, 280)}</p>
            {:else}<p class="muted">This note is not here.</p>{/if}
          {/if}
        </li>
      {/each}
    </ol>
  </div>

  {#if narrow}
    <form
      class="capture"
      onsubmit={(e) => {
        e.preventDefault();
        void capture();
      }}
    >
      <input
        type="text"
        bind:value={captured}
        placeholder="Jot into today, or a web address"
        aria-label="jot into today or open a web address"
        oninput={() => (captureNote = '')}
      />
      <button type="submit" disabled={!captured.trim()}>Add</button>
    </form>
    {#if captureNote}<p class="capture-note" role="status">{captureNote}</p>{/if}
  {/if}
</div>

<style>
  .home {
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .home.narrow {
    padding: var(--space-3) var(--space-3) var(--space-5);
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
  .edit {
    border: 1px dashed var(--accent);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .edit .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 3px 0;
    border-bottom: 1px dashed var(--border);
    cursor: grab;
  }
  .list .name {
    flex: 1;
    min-width: 8rem;
  }
  .list button {
    border: 1px solid var(--border);
    background: var(--bg);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .phone {
    font-size: var(--text-sm);
    white-space: nowrap;
  }
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .add .reset {
    margin-left: auto;
  }
  .words {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  .words textarea {
    font: inherit;
  }
  .glance,
  .watch {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: var(--space-2);
  }
  .glance {
    flex-wrap: wrap;
  }
  .glance button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 9rem;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-align: left;
  }
  .glance b {
    font-size: var(--text-xl);
  }
  .glance span {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  /* The watch row is one line that scrolls sideways (a phone keeps it to one line). */
  .watch {
    overflow-x: auto;
    flex-wrap: nowrap;
  }
  .watch button {
    white-space: nowrap;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .n {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  .words-shown {
    margin: 0;
    white-space: pre-line;
  }
  .main {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-3);
    align-items: start;
  }
  .main.with-journal {
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
  }
  .narrow .main.with-journal {
    grid-template-columns: 1fr;
  }
  .journal {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
    gap: var(--space-3);
  }
  .main.with-journal .cards {
    grid-template-columns: 1fr;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    min-width: 0;
  }
  .card.full {
    grid-column: 1 / -1;
  }
  .card h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }
  .title {
    font: inherit;
    font-weight: inherit;
    color: inherit;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .title:hover {
    text-decoration: underline;
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
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
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
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link.strong {
    font-weight: 600;
  }
  .why {
    font-size: var(--text-xs);
    color: var(--danger-fg);
    white-space: nowrap;
  }
  .figure {
    margin: 0 0 var(--space-2);
  }
  .figure b {
    font-size: var(--text-xl);
  }
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 3rem;
  }
  .bars span {
    flex: 1;
    min-height: 2px;
    background: var(--accent);
    border-radius: 2px 2px 0 0;
    opacity: 0.75;
  }
  .columns {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(10rem, 1fr);
    gap: var(--space-2);
    overflow-x: auto;
  }
  .column h4 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    text-transform: capitalize;
  }
  .chip-card {
    display: block;
    width: 100%;
    text-align: left;
    margin-bottom: 4px;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    font: inherit;
    font-size: var(--text-sm);
    color: inherit;
    cursor: pointer;
  }
  .more {
    align-self: flex-start;
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
    margin: var(--space-1) 0 0;
    white-space: pre-line;
  }
  .error {
    color: var(--danger-fg);
  }
  /* The capture bar stays at the bottom, within a thumb's reach. */
  .capture {
    position: sticky;
    bottom: 0;
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) 0;
    background: var(--bg);
    margin-top: auto;
  }
  .capture input {
    flex: 1;
    font: inherit;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: inherit;
  }
  .capture button {
    font: inherit;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg, #fff);
  }
  .capture-note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>

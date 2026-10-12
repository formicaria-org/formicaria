<script lang="ts">
  // How each exercise is done, as its source wrote it (`decisions.md#ui`, 2026-10-12, *how an
  // exercise is done is its source's own words*).
  //
  // Two bodies' text is carried word for word: the NHS website's exercises that need no equipment,
  // and the U.S. Army's drills, which also cover a bar, free weights and machines. **Nothing on a
  // card is this app's wording** except where it is filed (what it needs, which part of the body)
  // — and the card says whose every line is, as each source's terms ask. Videos are links: a
  // known body's own channel first, a trainer's only where none was found, and said to be so.
  import Icon from '../Icon.svelte';
  import { exerciseLibrary } from '../ipc';
  import type { ExerciseCard, ExerciseLibrary, ExerciseVideo } from '../types';

  interface Props {
    /** Add this exercise to the session in front. */
    onadd: (card: { name: string; region: string }) => void;
  }
  let { onadd }: Props = $props();

  let lib = $state<ExerciseLibrary | null>(null);
  let failed = $state('');
  $effect(() => {
    exerciseLibrary()
      .then((l) => (lib = l))
      .catch((e) => (failed = String(e)));
  });

  const NEEDS: { key: string; label: string; has: ExerciseCard['equipment'][] }[] = [
    { key: 'none', label: 'No equipment', has: ['none', 'chair or wall'] },
    { key: 'bar', label: 'A bar', has: ['bar'] },
    { key: 'weights', label: 'Free weights', has: ['free weights'] },
    { key: 'machine', label: 'Machines', has: ['machine'] },
    { key: 'other', label: 'Other', has: ['other'] },
  ];
  const AREAS: Record<string, string> = {
    'upper-push': 'Upper body, push',
    'upper-pull': 'Upper body, pull',
    'lower-push': 'Legs, push',
    'lower-pull': 'Legs, pull',
    trunk: 'Trunk',
    balance: 'Balance',
    flexibility: 'Stretching',
    'warm-up': 'Warm-up',
    'whole-body': 'Whole body',
  };
  let need = $state('none');
  let area = $state('');
  let seek = $state('');
  const having = $derived(NEEDS.find((n) => n.key === need)?.has ?? []);
  const shown = $derived.by(() => {
    const text = seek.trim().toLowerCase();
    return (lib?.exercises ?? []).filter(
      (e) =>
        having.includes(e.equipment) &&
        (!area || e.area === area) &&
        (!text || e.name.toLowerCase().includes(text)),
    );
  });
  /// The parts of the body that the equipment chosen has anything for, in the order above.
  const areas = $derived.by(() => {
    const got = new Set(
      (lib?.exercises ?? []).filter((e) => having.includes(e.equipment)).map((e) => e.area),
    );
    return Object.keys(AREAS).filter((a) => got.has(a));
  });

  let open = $state<ExerciseCard | null>(null);
  const sourceOf = (e: ExerciseCard) => lib?.sources.find((s) => s.id === e.source);
  /// `121026` → `12 October 2026`: the day the NHS text was copied, which its terms ask to be shown.
  function asAt(ddmmyy: string | undefined): string {
    if (!ddmmyy || !/^\d{6}$/.test(ddmmyy)) return '';
    const d = new Date(
      2000 + Number(ddmmyy.slice(4)),
      Number(ddmmyy.slice(2, 4)) - 1,
      Number(ddmmyy.slice(0, 2)),
    );
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
  }
  /// The films found for an exercise: of the exercise itself, then of the drill it is part of.
  /// **Their order is the file's** (`exercise-videos.json`): a known body's that is not a drill
  /// manual's first, a trainer whose qualification was confirmed next, the Army's own clips last
  /// — the owner's order (`decisions.md#data`, 2026-10-12). Which film goes with which card is in
  /// the file too.
  function videosFor(e: ExerciseCard): ExerciseVideo[] {
    const all = lib?.videos ?? [];
    return [
      ...all.filter((v) => v.cards?.includes(e.id)),
      ...all.filter((v) => v.groups?.includes(e.group)),
    ];
  }
  const REGIONS = ['upper-push', 'upper-pull', 'lower-push', 'lower-pull'];
  const shownAlready = (c: ExerciseCard, text: string) =>
    [c.about ?? '', c.start ?? '', ...c.steps, ...(c.notes ?? [])].some((t) => t.includes(text));
</script>

<div class="cards">
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  {#if open}
    {@const card = open}
    {@const src = sourceOf(card)}
    {@const vids = videosFor(card)}
    <button type="button" class="back" onclick={() => (open = null)}>
      <Icon name="chevronLeft" size={18} /> All exercises
    </button>
    {#snippet watch()}
      {#if vids.length}
        <h3>To watch</h3>
        <ul class="videos">
          {#each vids as v (v.url)}
            <li>
              <a href={v.url} target="_blank" rel="noreferrer noopener">{v.title}</a>
              <span class="meta">
                {#if v.groups?.length}the whole drill ·{/if}
                {#if v.tier === 'body' && v.confirmed}
                  {v.channel}, its own channel
                {:else if v.tier === 'body'}
                  {v.channel} — a channel under that name, not confirmed from its own website
                {:else if v.tier === 'expert'}
                  {v.channel} — a trainer’s channel;
                  {#if v.credential_url}
                    <a href={v.credential_url} target="_blank" rel="noreferrer noopener"
                      >{v.credential}</a
                    >
                  {:else}{v.credential}{/if}
                {:else}
                  {v.channel}, a trainer’s channel — no known body’s film was found
                {/if}
                {#if v.tier !== 'body' && v.views}
                  · {v.views.toLocaleString()} views when seen
                {/if}
              </span>
            </li>
          {/each}
        </ul>
        <p class="meta">Links to YouTube, seen {lib?.videos_seen ?? ''}. Nothing is played here.</p>
      {/if}
    {/snippet}
    <article class="card" aria-label={card.name}>
      <h2>{card.name}</h2>
      <p class="meta">{AREAS[card.area] ?? card.area} · {card.group}</p>
      <!-- A drill manual's steps read hard; where there is a film, it comes first. -->
      {#if card.source === 'army'}
        {@render watch()}
        <h3>Written steps, from the U.S. Army’s manual</h3>
      {/if}
      <!-- Each source's own text, set apart and credited as its terms ask. -->
      <div class="theirs">
        {#if card.about}<p>{card.about}</p>{/if}
        <!-- Said again only when it is not already a sentence of what is shown. -->
        {#if card.equipment_said && !shownAlready(card, card.equipment_said)}
          <p>{card.equipment_said}</p>
        {/if}
        {#if card.start}<p>{card.start}</p>{/if}
        <ol>
          {#each card.steps as step, i (i)}<li>{step}</li>{/each}
        </ol>
        {#if card.dose}<p class="dose">{card.dose}</p>{/if}
        {#each card.notes ?? [] as note, i (i)}<p>{note}</p>{/each}
        <p class="credit">
          <a href={card.url} target="_blank" rel="noreferrer noopener"
            >{src?.credit ?? ''}{src?.as_at ? `, as at ${asAt(src.as_at)}` : ''}</a
          >
          · {card.ref}.
          {#if src?.licence_url}
            <a href={src.licence_url} target="_blank" rel="noreferrer noopener">{src.licence}</a>.
          {:else if src}
            {src.licence}.
          {/if}
          {#if card.source === 'army'}
            Written for soldiers, and shown as written: the counts and commands are the Army’s.
          {/if}
        </p>
      </div>

      {#if card.source !== 'army'}{@render watch()}{/if}
      <button
        type="button"
        class="go"
        onclick={() =>
          onadd({ name: card.name, region: REGIONS.includes(card.area) ? card.area : '' })}
      >
        Add to today’s session
      </button>
    </article>
  {:else}
    <header class="head">
      <h2>Exercises</h2>
      <p class="meta">How each is done, in the words of the body that published it.</p>
    </header>
    <div class="chips" role="group" aria-label="what you have">
      {#each NEEDS.filter( (n) => (lib?.exercises ?? []).some( (e) => n.has.includes(e.equipment) ) ) as n (n.key)}
        <button
          type="button"
          class="chip"
          class:on={need === n.key}
          aria-pressed={need === n.key}
          onclick={() => ((need = n.key), (area = ''))}>{n.label}</button
        >
      {/each}
    </div>
    <div class="chips" role="group" aria-label="which part of the body">
      <button
        type="button"
        class="chip"
        class:on={!area}
        aria-pressed={!area}
        onclick={() => (area = '')}>All</button
      >
      {#each areas as a (a)}
        <button
          type="button"
          class="chip"
          class:on={area === a}
          aria-pressed={area === a}
          onclick={() => (area = a)}>{AREAS[a]}</button
        >
      {/each}
    </div>
    <input
      class="field"
      type="search"
      placeholder="Search exercises"
      aria-label="search exercises"
      bind:value={seek}
      spellcheck="false"
    />
    {#if lib && shown.length === 0}
      <p class="meta">None for that.</p>
    {/if}
    <ul class="list">
      {#each shown as e (e.id)}
        <li>
          <button type="button" class="row" onclick={() => (open = e)}>
            <span class="name">{e.name}</span>
            <span class="meta">{AREAS[e.area] ?? e.area} · {sourceOf(e)?.by ?? ''}</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if !lib && !failed}<p class="meta">Loading…</p>{/if}
  {/if}
</div>

<style>
  .cards {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-3) max(var(--space-3), calc((100% - 36rem) / 2)) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    scrollbar-width: thin;
  }
  .cards > :global(*) {
    flex-shrink: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  p {
    margin: 0;
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  .meta,
  .credit {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .field {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .list,
  .videos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .row {
    width: 100%;
    box-sizing: border-box;
    min-height: 3.25rem;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
  }
  .name {
    font-weight: 600;
  }
  .back {
    align-self: flex-start;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  /* The source's own words, visibly apart from the app's. */
  .theirs {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-left: var(--space-3);
    border-left: 3px solid var(--border);
  }
  .theirs ol {
    margin: 0;
    padding-left: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .dose {
    font-weight: 600;
  }
  .videos li {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .go {
    align-self: flex-start;
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .failed {
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
</style>

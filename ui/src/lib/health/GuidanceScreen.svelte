<script lang="ts">
  // **Guidance** (`decisions.md#ui` `#data`, 2026-10-22): the Health cards. One thing at a time,
  // as on every screen of a group: what is followed, counted today; or the shelves; or one card,
  // opened. A card is its sources' own words, badged by what it rests on, with what it does not say.
  // A check is the person's count beside a named body's number, never a word about the person.
  //
  // Following a card writes one note (`program: health`, `plan: <card>`) and adds the trackers it
  // reads to the person's Health template; stopping gives that note a last day. Measuring is the
  // core's (`guidance.rs`).
  import { untrack } from 'svelte';
  import ManualLink from '../ManualLink.svelte';
  import * as nav from '../nav.svelte';
  import { healthDays, healthGuidance } from '../ipc';
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { addDays, dayLabel } from '../units';
  import type { HealthCard, HealthCounted, HealthGuidance, Tracker } from '../types';
  import { TEMPLATE } from './starters';

  interface Props {
    today: string;
    signal: unknown;
    onsaved: () => void;
    /** The window's id: its card and form close on Back without touching another window's. */
    pane?: string;
  }
  let { today, signal, onsaved, pane = 'health' }: Props = $props();

  let data = $state<HealthGuidance | null>(null);
  let template = $state<string | null>(null);
  let trackers = $state<Tracker[]>([]);
  let failed = $state('');
  let busy = $state(false);
  let open = $state<HealthCard | null>(null);
  /** For a card that asks: which of the person's trackers go under each choice. */
  let picks = $state<Record<string, string[]>>({});

  async function load() {
    try {
      const [g, d] = await Promise.all([healthGuidance(today), healthDays(today, today)]);
      data = g;
      template = d.template;
      trackers = d.trackers;
      failed = '';
    } catch (e) {
      failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    untrack(load);
  });
  $effect(() => {
    if (open) nav.openLayer(`health-card:${pane}`, () => (open = null));
    else nav.closeLayer(`health-card:${pane}`);
  });
  $effect(() => () => nav.closeLayer(`health-card:${pane}`));

  const SHELVES: [HealthCard['shelf'], string][] = [
    ['diary', 'Keeping a diary'],
    ['nose', 'Nose and allergy'],
    ['daily', 'Every day'],
    ['cycle', 'Cycle'],
    ['mind', 'Mind'],
    ['own', 'Yours'],
  ];
  const BADGE: Record<HealthCard['rests_on'], string> = {
    guideline: 'Guideline',
    consensus: 'Expert consensus',
    review: 'Review',
    definition: 'Definition, not advice',
    own: 'Yours',
  };

  // ── a card of one's own ────────────────────────────────────────────────────────────────────
  /** How an own card may count (`guidance.rs`'s `OWN_KINDS`), in words. */
  const OWN_COUNTS = [
    { kind: 'week_days', label: 'Days this week with it' },
    { kind: 'month_days', label: 'Days this month with it' },
    { kind: 'days_over', label: 'Days this week above a number' },
    { kind: 'at_least', label: 'Days this week at a number or more' },
    { kind: 'run', label: 'Most days in a row, last 30 days' },
  ];
  let writing = $state(false);
  let own = $state({
    name: '',
    body: '',
    key: '',
    count: 'week_days',
    value: 1,
    aim: '',
    aimAt: 1,
  });
  $effect(() => {
    if (writing) nav.openLayer(`health-own:${pane}`, () => (writing = false));
    else nav.closeLayer(`health-own:${pane}`);
  });
  $effect(() => () => nav.closeLayer(`health-own:${pane}`));
  function startOwn() {
    own = {
      name: '',
      body: '',
      key: trackers[0]?.key ?? '',
      count: 'week_days',
      value: 1,
      aim: '',
      aimAt: 1,
    };
    writing = true;
  }
  async function saveOwn() {
    if (busy || !own.name.trim() || !own.key) return;
    busy = true;
    failed = '';
    try {
      const props: Record<string, string | number> = {
        program: 'health',
        plan: 'own',
        own_key: own.key,
        own_count: own.count,
      };
      if (own.count === 'days_over' || own.count === 'at_least') props.own_value = own.value;
      if (own.aim === 'min') props.own_min = own.aimAt;
      if (own.aim === 'max') props.own_max = own.aimAt;
      await createNotesUndoable(
        [{ title: own.name.trim(), body: own.body.trim(), start: today, props }],
        `wrote ${own.name.trim()}`,
      );
      onsaved();
      writing = false;
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
  const followedOf = (id: string) => data?.followed.find((f) => f.card === id);
  /** Followed cards in the shelves' order, the person's own last, as the library lists them. */
  const following = $derived(
    [...(data?.followed ?? [])].sort(
      (a, b) =>
        (data?.cards.findIndex((c) => c.id === a.card) ?? 0) -
        (data?.cards.findIndex((c) => c.id === b.card) ?? 0),
    ),
  );
  const cardOf = (id: string) => data?.cards.find((c) => c.id === id);
  const daysOf = (n: number) => (n === 1 ? '1 day' : `${n} days`);
  /** What the count is against: the body's number, as a phrase. */
  /** How a check counts, from its card: the screen words a count by it. */
  const kindOf = (card: HealthCard, c: HealthCounted) =>
    card.checks.find((x) => x.id === c.id)?.kind ?? '';
  function shownValue(card: HealthCard, c: HealthCounted): string {
    const kind = kindOf(card, c);
    if (kind === 'bp_home') return c.values?.length === 2 ? `${c.values[0]}/${c.values[1]}` : '—';
    if (kind === 'questionnaire') return c.not_compared ? '—' : String(c.value);
    if (kind === 'cycles_kept' || c.outside !== undefined) return `${c.value} of ${c.of}`;
    if (kind === 'food_mean' && c.not_compared && !c.recorded) return '—';
    return c.values?.length ? c.values.join(', ') : String(c.value);
  }
  /** A self-harm item answered other than "Not at all": the card's urgent contacts come first. */
  const urgentNow = (card: HealthCard, f: { checks: HealthCounted[] }) =>
    !!card.form &&
    f.checks.some(
      (c) => kindOf(card, c) === 'questionnaire' && (c.values?.[c.values.length - 1] ?? 0) > 0,
    );

  // ── a questionnaire on its card ───────────────────────────────────────────────────────────
  let answers = $state<Record<string, number>>({});
  let saved = $state('');
  async function saveForm(card: HealthCard) {
    if (busy || !card.form || Object.keys(answers).length === 0) return;
    busy = true;
    failed = '';
    try {
      const id = (await healthDays(today, today)).days[0]?.id;
      if (id) await setPropertiesUndoable(id, { ...answers }, `answered ${card.name}`);
      else
        await createNotesUndoable(
          [{ title: 'Health diary', start: today, props: { log: 'health', ...answers } }],
          `answered ${card.name}`,
        );
      const n = Object.keys(answers).length;
      saved = `Saved for today: ${n} of ${card.form.items.length} answered.`;
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  function against(c: HealthCounted): string {
    if (c.min !== undefined && c.max !== undefined) return `${c.min} to ${c.max}`;
    if (c.min !== undefined) return `at least ${c.min}`;
    if (c.max !== undefined) return `at most ${c.max}`;
    if (c.at_or_over !== undefined) return `${c.at_or_over} or more is the number named`;
    return '';
  }
  /** What the count was taken over, in the words that fit it. */
  function over(c: HealthCounted, card?: HealthCard): string {
    const kind = card ? kindOf(card, c) : '';
    if (kind === 'bp_home') return `readings on ${c.recorded} of the last 7 days`;
    if (kind === 'questionnaire')
      return c.not_compared ? '' : `answered ${dayLabel(c.from)}, out of 27`;
    if (kind === 'food_mean') return `logged in Food on ${c.recorded} of ${daysOf(c.of)}`;
    if (kind === 'cycles_kept') return `cycles of the last 6 months`;
    if (c.cycles) return `${c.recorded} of ${c.of} cycles counted`;
    if (c.outside !== undefined)
      return `of your last ${c.of} periods; ${daysOf(c.outside)} with a headache at other times`;
    return `recorded on ${c.recorded} of ${daysOf(c.of)}`;
  }
  /** A cycle's reason for being left out, in words. */
  const why = (left: string) =>
    left === 'gap' ? 'left out: marked as missed logging' : `left out: ${left}`;
  async function markGap(note: string, gap: boolean) {
    if (busy || !note) return;
    busy = true;
    try {
      await setPropertiesUndoable(
        note,
        { cycle_gap_yes: gap ? 1 : '' },
        gap ? 'marked a cycle as missed logging' : 'counted a cycle again',
      );
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
  /** The person's count trackers, offered where a card asks which are which. */
  const counts = $derived(trackers.filter((t) => t.kind === 'count'));

  function openCard(card: HealthCard) {
    const f = followedOf(card.id);
    picks = Object.fromEntries((card.choose ?? []).map((c) => [c.id, f?.chosen[c.id] ?? []]));
    answers = {};
    saved = '';
    open = card;
  }
  function togglePick(choice: string, key: string) {
    const now = picks[choice] ?? [];
    picks = {
      ...picks,
      [choice]: now.includes(key) ? now.filter((k) => k !== key) : [...now, key],
    };
  }

  async function follow(card: HealthCard) {
    if (busy) return;
    busy = true;
    failed = '';
    try {
      // The trackers it reads, added to the template if missing.
      const missing = card.trackers.filter((k) => !trackers.some((t) => t.key === k));
      if (missing.length) {
        const props = Object.fromEntries(missing.map((k) => [k, 0]));
        if (template) await setPropertiesUndoable(template, props, `tracking for ${card.name}`);
        else
          await createNotesUndoable(
            [
              {
                title: TEMPLATE.title,
                body: TEMPLATE.body,
                tags: TEMPLATE.tags,
                props: { group: TEMPLATE.group, ...props },
              },
            ],
            `tracking for ${card.name}`,
          );
      }
      const chosen = Object.fromEntries(
        Object.entries(picks).map(([id, keys]) => [`p_${id}`, keys.join(',')]),
      );
      const was = followedOf(card.id);
      if (was) await setPropertiesUndoable(was.note, chosen, `changed ${card.name}`);
      else
        await createNotesUndoable(
          [
            {
              title: card.name,
              start: today,
              props: { program: 'health', plan: card.id, ...chosen },
            },
          ],
          `followed ${card.name}`,
        );
      onsaved();
      open = null;
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  async function stop(card: HealthCard) {
    const f = followedOf(card.id);
    if (!f || busy) return;
    busy = true;
    try {
      // Its last day is yesterday, as in Fitness: stopping takes effect now.
      await setPropertiesUndoable(f.note, { due: addDays(today, -1) }, `stopped ${card.name}`);
      onsaved();
      open = null;
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="guidance">
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  {#if writing}
    <form
      class="card"
      aria-label="your own card"
      onsubmit={(e) => (e.preventDefault(), void saveOwn())}
    >
      <header>
        <button
          type="button"
          class="back"
          onclick={() => (writing = false)}
          aria-label="back to the cards">‹</button
        >
        <h2>Your own card</h2>
      </header>
      {#if trackers.length === 0}
        <p class="note">
          Track something first (＋ in Diary); a card counts one of the things you track.
        </p>
      {:else}
        <label class="field"
          ><span>Name</span><input
            type="text"
            bind:value={own.name}
            placeholder="Wash the bedding"
          /></label
        >
        <label class="field"
          ><span>In your own words (optional)</span><textarea
            bind:value={own.body}
            rows="3"
            placeholder="Why, and who said so"></textarea></label
        >
        <label class="field"
          ><span>It counts</span><select bind:value={own.key}>
            {#each trackers as t (t.key)}<option value={t.key}>{t.label}</option>{/each}
          </select></label
        >
        <label class="field"
          ><span>How</span><select bind:value={own.count}>
            {#each OWN_COUNTS as c (c.kind)}<option value={c.kind}>{c.label}</option>{/each}
          </select></label
        >
        {#if own.count === 'days_over' || own.count === 'at_least'}
          <label class="field"
            ><span>The number</span><input
              type="number"
              min="0"
              step="1"
              bind:value={own.value}
            /></label
          >
        {/if}
        <label class="field"
          ><span>Your aim (optional)</span><select bind:value={own.aim}>
            <option value="">None</option>
            <option value="min">At least</option>
            <option value="max">At most</option>
          </select></label
        >
        {#if own.aim}
          <label class="field"
            ><span>Days</span><input type="number" min="0" step="1" bind:value={own.aimAt} /></label
          >
        {/if}
        <p class="note">Your number is your own; the card says so, and needs no source.</p>
        <div class="buttons">
          <button type="submit" class="go" disabled={busy || !own.name.trim() || !own.key}
            >Follow it</button
          >
        </div>
      {/if}
    </form>
  {:else if open}
    {@const f = followedOf(open.id)}
    <article class="card" aria-label={open.name}>
      <header>
        <button
          type="button"
          class="back"
          onclick={() => (open = null)}
          aria-label="back to the cards">‹</button
        >
        <h2>{open.name}</h2>
        <span class="badge" data-rests={open.rests_on}>{BADGE[open.rests_on]}</span>
      </header>

      {#if open.urgent?.length}
        <div class="urgent" role="note">
          {#each open.urgent as u, i (i)}
            <p>{u.text}</p>
            <cite>{u.by}</cite>
          {/each}
        </div>
      {/if}

      {#each open.says as said, i (i)}
        <blockquote>
          <p>{said.text}</p>
          <cite>{said.by}</cite>
        </blockquote>
      {/each}
      {#if open.says.some((s) => s.nhs) || open.urgent?.some((s) => s.nhs)}
        <p class="note">{data?.nhs_credit}</p>
      {/if}
      {#if open.disagree}<p class="disagree">{open.disagree}</p>{/if}

      {#if open.checks.length}
        <h3>What it counts from your diary</h3>
        <ul class="plain">
          {#each open.checks as c (c.id)}<li><strong>{c.label}</strong> — {c.basis}</li>{/each}
        </ul>
      {/if}

      {#if open.form}
        {@const form = open.form}
        <h3>Fill it in</h3>
        <p class="note">{form.prompt}</p>
        <ol class="form">
          {#each form.items as item (item.key)}
            <li>
              <span>{item.text}</span>
              <div class="answers" role="group" aria-label={item.text}>
                {#each form.answers as word, n (n)}
                  <button
                    type="button"
                    class:on={answers[item.key] === n}
                    aria-pressed={answers[item.key] === n}
                    onclick={() => (answers = { ...answers, [item.key]: n })}>{word}</button
                  >
                {/each}
              </div>
            </li>
          {/each}
        </ol>
        <div class="buttons">
          <button
            type="button"
            class="go"
            disabled={busy || Object.keys(answers).length === 0}
            onclick={() => saveForm(open!)}>Save today's answers</button
          >
        </div>
        {#if saved}<p class="note" role="status">
            {saved} It is counted when every question is answered.
          </p>{/if}
      {/if}

      {#if open.choose?.length}
        <h3>Which of your trackers are which</h3>
        {#if counts.length === 0}
          <p class="note">
            Track each medicine you take as a count first (＋ in Diary, “How many times”), then
            choose here.
          </p>
        {:else}
          {#each open.choose as choice (choice.id)}
            <fieldset>
              <legend>{choice.label}</legend>
              {#each counts as t (t.key)}
                <label>
                  <input
                    type="checkbox"
                    checked={(picks[choice.id] ?? []).includes(t.key)}
                    onchange={() => togglePick(choice.id, t.key)}
                  />
                  {t.label}
                </label>
              {/each}
            </fieldset>
          {/each}
          <p class="note">
            The class of a medicine is yours to say; the app does not read it off a name.
          </p>
        {/if}
      {/if}

      {#if open.not_said.length}
        <h3>What this card does not say</h3>
        <ul class="plain">
          {#each open.not_said as n, i (i)}<li>{n}</li>{/each}
        </ul>
      {/if}

      <h3>Sources</h3>
      <ul class="plain sources">
        {#each open.sources as s, i (i)}
          <li><a href={s.url} target="_blank" rel="noopener noreferrer">{s.title}</a> — {s.by}</li>
        {/each}
      </ul>

      <div class="buttons">
        <button type="button" class="go" disabled={busy} onclick={() => follow(open!)}
          >{f ? 'Save' : 'Follow'}</button
        >
        {#if f}
          <button type="button" class="go quiet" disabled={busy} onclick={() => stop(open!)}
            >Stop following</button
          >
        {/if}
      </div>
    </article>
  {:else if data}
    {#if data.followed.length}
      <section aria-label="followed">
        <h2>Following</h2>
        {#each following as f (f.note)}
          {@const card = cardOf(f.card)}
          {#if card}
            <div class="followed">
              <button type="button" class="title" onclick={() => openCard(card)}>{card.name}</button
              >
              <span class="since">since {dayLabel(f.since)}</span>
              {#if urgentNow(card, f)}
                <div class="urgent" role="alert">
                  {#each card.urgent ?? [] as u, k (k)}<p>{u.text}</p>
                    <cite>{u.by}</cite>{/each}
                </div>
              {/if}
              {#each f.checks as c, i (c.id)}
                <div class="check">
                  <span class="label">{c.label}</span>
                  {#if c.unchosen}
                    <span class="note">Choose which of your trackers count, in the card.</span>
                  {:else}
                    <span class="value"
                      >{shownValue(card, c)}{#if c.within}<span
                          class="within"
                          aria-label="within the number"
                        >
                          ✓</span
                        >{/if}</span
                    >
                    <span class="note"
                      >{[against(c), over(c, card), c.basis].filter(Boolean).join(' · ')}</span
                    >
                    {#if c.not_compared}<span class="note">Not compared: {c.not_compared}.</span
                      >{/if}
                    {#if c.cycles?.length && f.checks.findIndex((x) => x.cycles?.length) === i}
                      <ul class="cycles" aria-label="cycles">
                        {#each c.cycles as cy (cy.start)}
                          <li>
                            <span
                              >From {dayLabel(cy.start)}: {cy.length} days, {cy.noted} with a note{cy.left_out
                                ? ` (${why(cy.left_out)})`
                                : ''}</span
                            >
                            {#if cy.left_out === 'gap' || !cy.left_out}
                              <button
                                type="button"
                                class="link"
                                disabled={busy}
                                onclick={() => markGap(cy.note, cy.left_out !== 'gap')}
                                >{cy.left_out === 'gap'
                                  ? 'Count it again'
                                  : 'Missed logging'}</button
                              >
                            {/if}
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  {/if}
                </div>
              {/each}
              {#if !card.checks.length}<span class="note">Nothing to count; open it to read.</span
                >{/if}
            </div>
          {/if}
        {/each}
      </section>
    {/if}

    <p class="manual"><ManualLink page="user/health.html" section="guidance" /></p>
    {#each SHELVES as [shelf, heading] (shelf)}
      <section aria-label={heading}>
        <h2>{heading}</h2>
        <div class="shelf">
          {#if shelf === 'own'}
            <button type="button" class="pick" onclick={startOwn}>
              <strong>Write your own</strong>
              <span class="badge" data-rests="own">Yours</span>
            </button>
          {/if}
          {#each data.cards.filter((c) => c.shelf === shelf) as card (card.id)}
            <button type="button" class="pick" onclick={() => openCard(card)}>
              <strong>{card.name}</strong>
              <span class="badge" data-rests={card.rests_on}>{BADGE[card.rests_on]}</span>
              {#if followedOf(card.id)}<span class="on">Following</span>{/if}
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .guidance {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4) var(--space-6);
    max-width: 40rem;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  h2 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-lg);
  }
  h3 {
    margin: var(--space-2) 0 0;
    font-size: var(--text-md);
  }
  .shelf {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .pick {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
  }
  .pick strong {
    flex: 1 1 100%;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .field input,
  .field select,
  .field textarea {
    min-height: 2.75rem;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }
  /* The manual's link, drawn as a link: the component's own button carries no style. */
  .manual {
    margin: 0;
  }
  .manual :global(.manual-link) {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .form {
    margin: 0;
    padding-left: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .form li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .answers {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .answers button {
    flex: 1 1 8rem;
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .answers button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  .badge {
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .on {
    font-size: var(--text-xs);
    color: var(--accent);
    font-weight: 600;
  }
  .followed {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-2);
  }
  .followed .title {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }
  .since {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .check {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0 var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .check .note {
    grid-column: 1 / -1;
  }
  .value {
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .within {
    color: var(--accent);
  }
  .cycles {
    grid-column: 1 / -1;
    margin: var(--space-1) 0 0;
    padding-left: var(--space-4);
    font-size: var(--text-sm);
  }
  .cycles li {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: baseline;
  }
  /* A finger needs the same 2.75rem the rest of the app gives it. */
  @media (pointer: coarse) {
    .link,
    .followed .title {
      min-height: 2.75rem;
    }
  }
  @media (max-width: 40rem) {
    .guidance {
      padding-left: var(--space-3);
      padding-right: var(--space-3);
    }
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .card header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .card header h2 {
    margin: 0;
    flex: 1;
  }
  .back {
    width: 2.75rem;
    height: 2.75rem;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    font-size: var(--text-xl);
    cursor: pointer;
  }
  blockquote {
    margin: 0;
    padding-left: var(--space-3);
    border-left: 3px solid var(--border-strong);
  }
  blockquote p,
  .urgent p {
    margin: 0;
  }
  cite {
    display: block;
    font-size: var(--text-xs);
    font-style: normal;
    color: var(--text-muted);
  }
  .urgent {
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--danger-bg);
    color: var(--danger-fg);
  }
  .disagree {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-hover);
    font-size: var(--text-sm);
  }
  .plain {
    margin: 0;
    padding-left: var(--space-4);
    font-size: var(--text-sm);
  }
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  legend {
    font-size: var(--text-sm);
  }
  .note {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
  }
  .go {
    min-height: 3rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .go.quiet {
    background: var(--surface-hover);
    color: var(--text);
  }
  .go:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .failed {
    margin: 0;
    color: var(--danger-fg);
  }
</style>

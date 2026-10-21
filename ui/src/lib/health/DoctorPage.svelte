<script lang="ts">
  // **For the doctor** (`decisions.md#ui`, 2026-10-22): a page of the diary to print or to save as
  // a CSV — the counts first, then every day. People misjudge their own month by four to six days
  // of headache or medication (van Casteren 2021), so the counts lead; clinicians take to short
  // printouts (Epstein 2017). The person picks which trackers go on it. It is a record the person
  // kept, and says so; it names no condition.
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { healthDays } from '../ipc';
  import { isPhone } from '../platform';
  import { createNotesUndoable } from '../undo.svelte';
  import { addDays, dayLabel } from '../units';
  import type { HealthDays, Tracker } from '../types';
  import { BOTHER_CREDIT, SEVERITY, UNITS, WORDS } from './starters';

  interface Props {
    /** The first day of the month the page opens on. */
    month: string;
    today: string;
    onclose: () => void;
  }
  let { month, today, onclose }: Props = $props();

  type Span = 'month' | 'weeks8';
  let span = $state<Span>('month');
  let data = $state<HealthDays | null>(null);
  let failed = $state('');
  let left = $state<string[]>([]);

  const endOf = (first: string) => addDays(addDays(first, 32).slice(0, 8) + '01', -1);
  /** NICE CG150 1.1.4: a headache diary *"for a minimum of 8 weeks"*. */
  const range = $derived(
    span === 'month'
      ? [month, endOf(month) < today ? endOf(month) : today]
      : [addDays(today, -55), today],
  );

  $effect(() => {
    const [from, to] = range;
    untrack(async () => {
      try {
        data = await healthDays(from, to);
        failed = '';
      } catch (e) {
        failed = String(e);
      }
    });
  });

  // While this page is open, printing prints it alone (the rule is in this file's styles, keyed
  // to the class, so printing anything else is untouched).
  $effect(() => {
    document.body.classList.add('fm-print-doctor');
    return () => document.body.classList.remove('fm-print-doctor');
  });

  const shown = $derived((data?.trackers ?? []).filter((t) => !left.includes(t.key)));
  const unitLabel = (u: string) => UNITS.find((x) => x.unit === u)?.label ?? u;
  const heading = (t: Tracker) =>
    t.kind === 'amount' ? `${t.label} (${unitLabel(t.unit)})` : t.label;
  const cell = (t: Tracker, v: number | undefined) =>
    v === undefined
      ? ''
      : t.kind === 'severity'
        ? (WORDS[t.key] ?? SEVERITY)[v]
        : t.kind === 'yesno'
          ? v
            ? 'Yes'
            : 'No'
          : String(v);
  const daysOf = (n: number) => (n === 1 ? '1 day' : `${n} days`);
  const period = $derived(`${dayLabel(range[0])} – ${dayLabel(range[1])}`);
  const late = $derived(data?.days.some((d) => d.late) ?? false);

  function toggle(key: string) {
    left = left.includes(key) ? left.filter((k) => k !== key) : [...left, key];
  }

  /** The same numbers as a CSV: one row a day, one column a tracker, empty where not recorded. */
  const csv = $derived.by(() => {
    if (!data) return '';
    const q = (s: string) => (/[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);
    const head = ['day', ...shown.map((t) => t.key), 'written_later'];
    const rows = data.days.map((d) => [
      d.day,
      ...shown.map((t) => (d.values[t.key] === undefined ? '' : String(d.values[t.key]))),
      d.late ? '1' : '',
    ]);
    return [head, ...rows].map((r) => r.map(q).join(',')).join('\n') + '\n';
  });

  // **On a phone, the page is kept as a note.** Android's WebView prints and downloads only when the
  // shell handles them, and formicaria's does not, so Print and Save as CSV are not offered there
  // (unverified on a device: shown again once tried). A note works on every device, and a
  // computer can print it or a backup carry it.
  const phone = isPhone();
  let kept = $state('');
  const md = (cells: string[]) => `| ${cells.map((c) => c.replace(/\|/g, '\\|')).join(' | ')} |`;
  const markdown = $derived.by(() => {
    if (!data) return '';
    const counts = shown.map((t) => {
      const c = data!.counts[t.key];
      return md([
        heading(t),
        `${c.recorded} of ${daysOf(c.of)}`,
        t.kind === 'yesno' ? `Yes on ${daysOf(c.days)}` : daysOf(c.days),
        c.moderate_or_severe === undefined ? '' : daysOf(c.moderate_or_severe),
      ]);
    });
    const days = data.days.map((d) =>
      md([
        `${dayLabel(d.day)}${d.late ? ' *' : ''}`,
        ...shown.map((t) => cell(t, d.values[t.key])),
      ]),
    );
    return [
      period,
      '',
      md(['', 'Recorded', 'At 1 or more', 'At 2 or more']),
      md(['---', '---', '---', '---']),
      ...counts,
      '',
      md(['Day', ...shown.map(heading)]),
      md(['---', ...shown.map(() => '---')]),
      ...days,
      '',
      `A blank is a day not recorded.${late ? ' * Written in more than a day later.' : ''}`,
      'Kept by the person, day by day, in formicaria. A record, not a diagnosis.',
    ].join('\n');
  });
  async function keep() {
    try {
      const [made] = await createNotesUndoable(
        [{ title: `Health diary, ${period}`, body: markdown, tags: ['health'], props: {} }],
        'kept the page for the doctor',
      );
      kept = made.title ?? 'the note';
    } catch (e) {
      failed = String(e);
    }
  }

  function save() {
    if (typeof URL.createObjectURL !== 'function') return;
    const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `health-diary-${range[0]}-to-${range[1]}.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="doctor-overlay">
  <div class="doctor" role="dialog" aria-modal="true" aria-label="for the doctor">
    <header class="actions">
      <h2>For the doctor</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="actions controls">
      <div class="seg" role="group" aria-label="which days">
        <button
          type="button"
          class:on={span === 'month'}
          aria-pressed={span === 'month'}
          onclick={() => (span = 'month')}>This month</button
        >
        <button
          type="button"
          class:on={span === 'weeks8'}
          aria-pressed={span === 'weeks8'}
          onclick={() => (span = 'weeks8')}>Last 8 weeks</button
        >
      </div>
      {#if data && data.trackers.length}
        <fieldset>
          <legend>On the page</legend>
          {#each data.trackers as t (t.key)}
            <label
              ><input
                type="checkbox"
                checked={!left.includes(t.key)}
                onchange={() => toggle(t.key)}
              />
              {t.label}</label
            >
          {/each}
        </fieldset>
      {/if}
      <div class="buttons">
        {#if !phone}
          <button type="button" class="go" disabled={!shown.length} onclick={() => window.print()}
            >Print</button
          >
          <button type="button" class="go quiet" disabled={!shown.length} onclick={save}
            >Save as CSV</button
          >
        {/if}
        <button type="button" class="go quiet" disabled={!shown.length} onclick={keep}
          >Keep as a note</button
        >
      </div>
      {#if kept}<p class="note" role="status">Kept as a note: {kept}.</p>{/if}
    </div>

    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

    {#if data}
      <article class="page">
        <h1>Health diary</h1>
        <p class="period">{period}</p>
        {#if shown.length === 0}
          <p class="note">Nothing chosen to show.</p>
        {:else}
          <table class="summary">
            <thead>
              <tr><th></th><th>Recorded</th><th>At 1 or more</th><th>At 2 or more</th></tr>
            </thead>
            <tbody>
              {#each shown as t (t.key)}
                {@const c = data.counts[t.key]}
                <tr>
                  <th scope="row">{heading(t)}</th>
                  <td>{c.recorded} of {daysOf(c.of)}</td>
                  <td>{t.kind === 'yesno' ? `Yes on ${daysOf(c.days)}` : daysOf(c.days)}</td>
                  <td>{c.moderate_or_severe === undefined ? '' : daysOf(c.moderate_or_severe)}</td>
                </tr>
              {/each}
            </tbody>
          </table>

          <table class="days">
            <thead>
              <tr>
                <th>Day</th>
                {#each shown as t (t.key)}<th>{heading(t)}</th>{/each}
              </tr>
            </thead>
            <tbody>
              {#each data.days as d (d.day)}
                <tr>
                  <th scope="row">{dayLabel(d.day)}{d.late ? ' *' : ''}</th>
                  {#each shown as t (t.key)}<td>{cell(t, d.values[t.key])}</td>{/each}
                </tr>
              {/each}
            </tbody>
          </table>

          <footer>
            <p>
              A blank is a day not recorded. {late ? '* Written in more than a day later.' : ''}
            </p>
            {#if shown.some((t) => t.kind === 'severity')}
              <p>
                None, mild, moderate, severe: the International Headache Society's four steps
                (Diener 2019).
              </p>
            {/if}
            {#if shown.some((t) => t.kind === 'bother')}<p>{BOTHER_CREDIT}</p>{/if}
            <p>Kept by the person, day by day, in formicaria. A record, not a diagnosis.</p>
          </footer>
        {/if}
      </article>
    {/if}
  </div>
</div>

<style>
  .doctor-overlay {
    position: fixed;
    inset: 0;
    z-index: 85;
    display: flex;
    justify-content: center;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: var(--safe-top);
    background: var(--surface);
  }
  .doctor {
    width: min(48rem, 100%);
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom), var(--space-6));
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
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
  .controls {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .seg {
    display: flex;
    gap: var(--space-1);
  }
  .seg button {
    flex: 1;
    min-height: 2.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .seg button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  fieldset {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-4);
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  legend {
    font-size: var(--text-sm);
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
  .page h1 {
    margin: 0;
    font-size: var(--text-xl);
  }
  .period {
    margin: 0 0 var(--space-3);
    color: var(--text-muted);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: var(--space-4);
    font-size: var(--text-sm);
  }
  th,
  td {
    padding: var(--space-1) var(--space-2);
    border-bottom: 1px solid var(--border);
    text-align: left;
    white-space: nowrap;
  }
  .days {
    display: block;
    overflow-x: auto;
  }
  .buttons {
    flex-wrap: wrap;
  }
  @media (max-width: 40rem) {
    .summary th,
    .summary td {
      white-space: normal;
    }
    .doctor {
      padding-left: var(--space-3);
      padding-right: var(--space-3);
    }
  }
  footer p,
  .note {
    margin: 0 0 var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .failed {
    margin: 0;
    color: var(--danger-fg);
  }
  @media print {
    :global(body.fm-print-doctor *) {
      visibility: hidden;
    }
    :global(body.fm-print-doctor) .doctor-overlay,
    :global(body.fm-print-doctor) .doctor-overlay :global(*) {
      visibility: visible;
    }
    .doctor-overlay {
      position: absolute;
      inset: 0 auto auto 0;
      height: auto;
      width: 100%;
      padding: 0;
      background: white;
      color: black;
    }
    .doctor {
      overflow: visible;
      width: 100%;
      padding: 0;
    }
    .actions {
      display: none;
    }
    .days {
      display: table;
    }
    th,
    td {
      border-bottom-color: #999;
    }
  }
</style>

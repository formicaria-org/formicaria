<script lang="ts">
  // A week to start from (`decisions.md#data`, 2026-10-12, *every regime has a week to start
  // from*): a body's own example where it publishes one, and otherwise the app's arrangement of
  // the regime's figures and the exercise cards — and the screen says which. **Only a simple
  // indication, to be changed**: using it makes ordinary routines, which are the person's.
  import { exercises } from '../training/exercises';
  import { createNotesUndoable } from '../undo.svelte';
  import { itemLine, routinesFrom } from './example';
  import type { ExerciseCard, PlanExample } from '../types';

  interface Props {
    example: PlanExample;
    onsaved: () => void;
  }
  let { example, onsaved }: Props = $props();

  let cards = $state<ExerciseCard[]>([]);
  $effect(() => {
    void exercises()
      .then((l) => (cards = l.exercises))
      .catch(() => (cards = []));
  });
  const DAYS: Record<string, string> = {
    mon: 'Monday',
    tue: 'Tuesday',
    wed: 'Wednesday',
    thu: 'Thursday',
    fri: 'Friday',
    sat: 'Saturday',
    sun: 'Sunday',
  };
  let busy = $state(false);
  let said = $state('');
  let failed = $state('');
  async function use() {
    if (busy) return;
    busy = true;
    failed = '';
    try {
      const drafts = routinesFrom(example, cards, Date.now().toString(36));
      await createNotesUndoable(drafts, 'made routines from an example week');
      said = 'Done: they are your routines now, on your week. Change or remove any of them there.';
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="example" aria-label="a week to start from">
  <h3>A week to start from{example.variant ? ` — ${example.variant}` : ''}</h3>
  <p class="note">{example.note}</p>
  <ul>
    {#each example.week as d (d.day)}
      <li>
        <span class="day">{DAYS[d.day] ?? d.day}</span>
        <span class="what">
          {#each d.items as item, i (i)}<span>{itemLine(item, cards)}</span>{:else}—{/each}
        </span>
      </li>
    {/each}
  </ul>
  {#if example.later}
    <details>
      <summary>After that</summary>
      <p class="note">{example.later}</p>
    </details>
  {/if}
  <p class="note">
    {example.kind === 'published' ? 'As published by' : 'Arranged by'}
    {example.by} ·
    <a href={example.url} target="_blank" rel="noreferrer noopener">{example.title}</a>
  </p>
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  {#if said}
    <p class="note" role="status">{said}</p>
  {:else}
    <button type="button" class="go" onclick={use} disabled={busy}>Use it as my week</button>
  {/if}
</section>

<style>
  a {
    color: var(--accent);
  }
  .example {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  .note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  li {
    display: grid;
    grid-template-columns: 5.5rem 1fr;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .day {
    color: var(--text-muted);
  }
  .what {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  summary {
    min-height: 2.25rem;
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .go {
    align-self: flex-start;
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .go:disabled {
    opacity: 0.5;
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
</style>

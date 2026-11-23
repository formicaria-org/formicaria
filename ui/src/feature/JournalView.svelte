<script lang="ts">
  // **The journal** (Home, 2026-11-23; `unifying-home-research-2026-11-22.md` §12.3): a day's entry,
  // ready to write in; under today's, **what the day collected** (planned and due, logged, came in,
  // written: what the Today page was); then the earlier days. Any day can be chosen, and its entry
  // is made on the first keystroke like today's.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import type { ObjectMeta } from '../lib/types';
  import { appToday, formatMoment } from '../lib/appTime';
  import { addDays } from '../lib/civil';
  import JournalEntry from './JournalEntry.svelte';
  import TodayView from './TodayView.svelte';

  let { onopen }: { onopen: (id: string) => void } = $props();

  const today = appToday();
  let day = $state(today);
  let earlier = $state<ObjectMeta[]>([]);
  let entry = $state<{ focus: () => void } | undefined>(undefined);

  /** The day before `d`, by the calendar (`civil.ts`). */
  const before = (d: string): string => addDays(d, -1);
  async function read() {
    earlier = await ipc.journal(before(day), 14).catch(() => []);
  }
  $effect(() => {
    void day;
    void read();
  });
  onMount(() => {
    const again = () => {
      day = today;
      entry?.focus();
    };
    addEventListener('fm-journal-today', again);
    const stop = ipc.listenChanges(() => void read());
    return () => {
      removeEventListener('fm-journal-today', again);
      stop();
    };
  });
  const short = (d: string) =>
    formatMoment(`${d}T12:00:00Z`, { weekday: 'short', day: 'numeric', month: 'short' });
</script>

<div class="journal">
  <div class="col">
    <header class="top">
      <h1>Journal</h1>
      {#if day !== today}
        <button type="button" class="today-btn" onclick={() => (day = today)}>Today</button>
      {/if}
      <label class="pick"
        >Go to a day <input
          type="date"
          aria-label="go to a day"
          value={day}
          max={today}
          onchange={(e) => (day = (e.currentTarget as HTMLInputElement).value || today)}
        /></label
      >
    </header>
    {#key day}
      <JournalEntry bind:this={entry} {day} {onopen} />
    {/key}
    {#if day === today}
      <section class="collected" aria-label="what the day collected">
        <h2>What the day collected</h2>
        <TodayView {onopen} heading="" />
      </section>
    {/if}
    <section class="earlier" aria-label="earlier days">
      <h2>Earlier days</h2>
      {#if earlier.length}
        <ul>
          {#each earlier as n (n.id)}
            <li>
              <button type="button" onclick={() => (day = n.title ?? today)}>
                <span class="d">{short(n.title ?? '')}</span>
                <span class="p">{n.preview}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">No earlier entries yet.</p>
      {/if}
    </section>
  </div>
</div>

<style>
  /* The page scrolls across its whole width; the reading column inside it is held to a width. */
  .journal {
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  .col {
    max-width: 52rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
  }
  .pick {
    margin-left: auto;
    font-size: var(--text-sm);
    color: var(--text-muted);
    display: inline-flex;
    gap: var(--space-2);
    align-items: center;
  }
  .today-btn {
    font: inherit;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  h2 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .earlier li button {
    display: flex;
    gap: var(--space-3);
    width: 100%;
    text-align: left;
    font: inherit;
    color: inherit;
    background: none;
    border: none;
    border-bottom: 1px dashed var(--border);
    padding: var(--space-1) 0;
    cursor: pointer;
  }
  .d {
    flex: none;
    width: 7rem;
    color: var(--text-muted);
  }
  .p {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: var(--text-muted);
    margin: 0;
  }
</style>

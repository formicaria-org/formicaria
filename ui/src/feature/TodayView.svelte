<script lang="ts">
  // **Today** (the shell, P2b; `unifying-home-research-2026-11-22.md` §9, D4): one line of time for the
  // day: what is planned and due, what was logged, what came in, what was written. Composed from the
  // reads that exist (the agenda, the recent notes), so it costs the server nothing new.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import type { ObjectMeta } from '../lib/types';
  import { appToday, dayOfInstant } from '../lib/appTime';

  let { onopen }: { onopen: (id: string) => void } = $props();

  let agenda = $state<ObjectMeta[]>([]);
  let recent = $state<ObjectMeta[]>([]);
  async function read() {
    [agenda, recent] = await Promise.all([
      ipc.getAgenda().catch(() => []),
      ipc.recent().catch(() => []),
    ]);
  }
  onMount(() => {
    void read();
    return ipc.listenChanges(() => void read());
  });

  const today = $derived(appToday());
  const planned = $derived(
    agenda.filter((n) => dayOfInstant(n.start ?? '') === today || (!!n.due && n.due <= today)),
  );
  const logged = $derived(
    recent.filter((n) => n.kind_key === 'log' && dayOfInstant(n.start ?? n.created) === today),
  );
  const cameIn = $derived(
    recent.filter(
      (n) =>
        ['web', 'song', 'video', 'meeting', 'paper'].includes(n.kind_key ?? '') &&
        dayOfInstant(n.created) === today,
    ),
  );
  const written = $derived(
    recent.filter(
      (n) =>
        !['log', 'web', 'song', 'video'].includes(n.kind_key ?? '') &&
        dayOfInstant(n.updated) === today,
    ),
  );
  const groups = $derived([
    { label: 'Planned and due', items: planned, empty: 'Nothing planned or due today.' },
    { label: 'Logged', items: logged, empty: 'Nothing logged yet today.' },
    { label: 'Came in', items: cameIn, empty: 'Nothing brought in today.' },
    { label: 'Written', items: written, empty: 'Nothing written today yet.' },
  ]);
</script>

<div class="today">
  <h1>Today</h1>
  {#each groups as g (g.label)}
    <section aria-label={g.label}>
      <h2>{g.label} <span class="n">{g.items.length}</span></h2>
      {#if g.items.length}
        <ul>
          {#each g.items as n (n.id)}
            <li>
              <button type="button" onclick={() => onopen(n.id)}>{n.title || n.preview}</button>
              {#if n.start && dayOfInstant(n.start) === today}<span class="when"
                  >{n.start.slice(11, 16)}</span
                >{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">{g.empty}</p>
      {/if}
    </section>
  {/each}
</div>

<style>
  .today {
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
    max-width: 52rem;
  }
  h1 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-2xl);
  }
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    margin-bottom: var(--space-3);
  }
  h2 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }
  .n {
    color: var(--text-muted);
    font-weight: 400;
    font-size: var(--text-sm);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    justify-content: space-between;
    padding: 3px 0;
    border-bottom: 1px dashed var(--border);
  }
  li button {
    font: inherit;
    background: none;
    border: none;
    padding: 0;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }
  li button:hover {
    text-decoration: underline;
  }
  .when,
  .muted {
    color: var(--text-muted);
  }
  .muted {
    margin: 0;
  }
</style>

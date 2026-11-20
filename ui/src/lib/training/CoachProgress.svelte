<script lang="ts">
  // **Progress, per sport** (Spec K.2, K4): each week's sessions planned and done, minutes planned
  // and done, and distance. Measured facts only — no load, no zone, no line across sports; those
  // wait for their own decisions.
  import { untrack } from 'svelte';
  import { coachProgress } from '../ipc';
  import { dayLabel } from '../units';
  import { sportLabel } from './coach';
  import type { CoachWeekProgress } from '../types';

  interface Props {
    today: string;
    signal?: unknown;
  }
  let { today, signal }: Props = $props();

  let weeks = $state<CoachWeekProgress[]>([]);
  let failed = $state('');
  $effect(() => {
    void signal;
    untrack(() => {
      coachProgress(today, 8)
        .then((w) => {
          weeks = w;
          failed = '';
        })
        .catch((e) => (failed = String(e)));
    });
  });
  const sports = $derived([...new Set(weeks.flatMap((w) => w.sports.map((s) => s.sport)))]);
  const of = (w: CoachWeekProgress, sport: string) => w.sports.find((s) => s.sport === sport);
  const km = (m: number) => (m ? `${Math.round(m / 100) / 10}` : '');
</script>

<section class="card" aria-label="Progress">
  <h3>Progress</h3>
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  {#if !sports.length}
    <p class="note">Nothing planned or done in the last eight weeks.</p>
  {/if}
  {#each sports as sport (sport)}
    <div class="scroll">
      <table>
        <caption>{sportLabel(sport)}</caption>
        <thead>
          <tr>
            <th scope="col">Week of</th>
            <th scope="col">Sessions done / planned</th>
            <th scope="col">Minutes done / planned</th>
            {#if sport !== 'strength'}<th scope="col">Km</th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each weeks as w (w.from)}
            {@const s = of(w, sport)}
            <tr>
              <th scope="row">{dayLabel(w.from)}</th>
              <td
                >{s ? `${s.done} / ${s.planned}` : '—'}{s?.skipped
                  ? ` (${s.skipped} skipped)`
                  : ''}</td
              >
              <td>{s ? `${s.done_minutes} / ${s.planned_minutes}` : '—'}</td>
              {#if sport !== 'strength'}<td>{s ? km(s.distance_m) : ''}</td>{/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/each}
  <p class="note">
    Planned counts what was put on the calendar; a session done without a plan counts as done only.
  </p>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  h3,
  p {
    margin: 0;
  }
  .scroll {
    overflow-x: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }
  caption {
    text-align: left;
    font-weight: 600;
    padding-bottom: var(--space-1);
  }
  th,
  td {
    padding: var(--space-1) var(--space-2);
    border-bottom: 1px solid var(--border);
    text-align: left;
    white-space: nowrap;
  }
  td {
    font-variant-numeric: tabular-nums;
  }
  .note {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .failed {
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
</style>

<script lang="ts">
  // **A planned session, out to a watch** (`decisions.md` 2026-11-20, *Fitness gets a Coach tab*,
  // item 6; the coach plan's S.7). The core writes the FIT workout file (`workout_fit`); here it is
  // saved — a download on a computer, Android's *Save as* on the phone — or, on the phone, handed to
  // the watch's own app through Android's share sheet. formicaria never talks to a watch: an app such
  // as Gadgetbridge installs the file, and it lands in the watch's Workouts list, not its calendar.
  import { workoutFit } from '../ipc';
  import { canSendWorkout, saveWorkout, sendWorkout } from '../watchFiles.svelte';
  import type { WorkoutFile } from '../types';

  let { id }: { id: string } = $props();

  let busy = $state(false);
  let failed = $state('');
  let notes = $state<string[]>([]);
  let said = $state('');
  const phone = canSendWorkout();

  async function out(how: 'save' | 'send') {
    if (busy) return;
    busy = true;
    failed = '';
    said = '';
    try {
      const f: WorkoutFile = await workoutFit(id);
      notes = f.notes;
      if (how === 'send' && sendWorkout(f)) {
        said =
          'Pick your watch app (Gadgetbridge, for example). The workout goes to the Workouts list on the watch, not its calendar.';
      } else {
        saveWorkout(f);
        said = `Saved as ${f.name}.`;
      }
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="out">
  <button type="button" disabled={busy} onclick={() => void out('save')}> Save .fit file </button>
  {#if phone}
    <button type="button" disabled={busy} onclick={() => void out('send')}>
      Send to watch app
    </button>
  {/if}
  <p class="note">
    {phone
      ? 'A workout file for the watch: its app installs it in the watch’s Workouts list, not its calendar.'
      : 'A workout file for the watch: copy it to the watch, or hand it to the watch’s app; it goes to the Workouts list, not the calendar.'}
    Not yet tried on every watch.
  </p>
  {#if said}<p class="note" aria-live="polite">{said}</p>{/if}
  {#each notes as n, i (i)}<p class="note">{n}</p>{/each}
  {#if failed}<p class="err" role="alert">{failed}</p>{/if}
</div>

<style>
  button {
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .out {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }
  .note {
    flex-basis: 100%;
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .err {
    flex-basis: 100%;
    margin: 0;
    color: var(--danger-fg);
  }
</style>

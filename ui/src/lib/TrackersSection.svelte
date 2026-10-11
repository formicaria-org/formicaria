<script lang="ts">
  // Food and training: whether this device shows them, and what the food list is.
  //
  // The list comes with the app (`decisions.md#toolchain`, 2026-10-11), so there is nothing to get
  // and nothing to remove — only something to say: what it is, whose it is, and what it takes.
  import { onMount } from 'svelte';
  import { packStatus } from './ipc';
  import { humanSize } from './size';
  import { showTrackers, trackersShown } from './trackers.svelte';
  import type { PackStatus } from './types';

  let status = $state<PackStatus | null>(null);
  onMount(async () => {
    try {
      status = await packStatus();
    } catch (e) {
      status = { installed: null, bytes: 0, error: e instanceof Error ? e.message : String(e) };
    }
  });
</script>

<section>
  <h3>Food and training</h3>
  <label class="switch-line">
    <input
      type="checkbox"
      role="switch"
      checked={trackersShown()}
      onchange={(e) => showTrackers((e.currentTarget as HTMLInputElement).checked)}
    />
    <span>Show Food and Training on this device</span>
  </label>
  <p class="muted">
    A day of meals against a target, and training sessions, among your views. What you log is kept
    as notes in your notebook, and stays out of the board, the agenda and the timeline.
  </p>

  {#if status?.installed}
    <p class="muted">
      Foods are looked up in a list that comes with the app: {status.installed.foods.toLocaleString()}
      foods, {humanSize(status.bytes)} on this device, no connection needed. It is made from two European
      food tables, with a third used for what they do not have.
      {status.installed.attribution}
      {status.installed.licence}.
    </p>
  {:else if status?.error}
    <p class="muted err" role="alert">
      The food list could not be put in place, so foods can only be added by name. {status.error}
    </p>
  {/if}
</section>

<style>
  section {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }
  .muted {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin: var(--space-2) 0 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .err {
    color: var(--danger-fg);
  }
  .switch-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    cursor: pointer;
  }
</style>

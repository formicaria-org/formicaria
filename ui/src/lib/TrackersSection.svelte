<script lang="ts">
  // Food and training: whether this device shows them, and the food list they look things up in.
  //
  // **Nothing here is spent unasked.** The list is a file of about eight megabytes; this says its
  // name, its size and its licence before the control that fetches it, and the control that removes
  // it says what comes back (`decisions.md#toolchain`, 2026-10-11). What you have already logged
  // keeps its numbers either way — each entry carries its own.
  import { onDestroy, onMount } from 'svelte';
  import { packDownload, packProgress, packRemove, packStatus } from './ipc';
  import { humanSize } from './size';
  import { showTrackers, trackersShown } from './trackers.svelte';
  import type { PackProgress, PackStatus } from './types';

  let status = $state<PackStatus | null>(null);
  let progress = $state<PackProgress | null>(null);
  let error = $state('');
  let armed = $state(false);
  let timer: ReturnType<typeof setInterval> | null = null;

  function sentence(e: unknown): string {
    const raw = e instanceof Error ? e.message : String(e);
    try {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.error === 'string') return parsed.error;
    } catch {
      /* already a sentence */
    }
    return raw;
  }

  async function refresh() {
    try {
      [status, progress] = await Promise.all([packStatus(), packProgress()]);
      if (progress?.error) error = progress.error;
    } catch (e) {
      error = sentence(e);
    }
    watch();
  }
  /// Ask again while a download runs, and stop asking the moment it does not.
  function watch() {
    if (progress?.running && !timer) timer = setInterval(refresh, 700);
    if (!progress?.running && timer) {
      clearInterval(timer);
      timer = null;
    }
  }
  onMount(refresh);
  onDestroy(() => timer && clearInterval(timer));

  async function download() {
    error = '';
    try {
      progress = await packDownload();
    } catch (e) {
      error = sentence(e);
    }
    // A fetch that is already over needs the status read again, or the row still says "get it".
    await refresh();
  }
  async function cancel() {
    try {
      progress = await packDownload(true);
    } catch (e) {
      error = sentence(e);
    }
  }
  async function remove() {
    error = '';
    armed = false;
    try {
      status = await packRemove();
    } catch (e) {
      error = sentence(e);
    }
  }

  const part = $derived(
    progress?.total ? Math.round((progress.received / progress.total) * 100) : null,
  );
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

  {#if status}
    <h4>{status.title}</h4>
    {#if status.installed}
      <p class="muted">
        On this device: {status.installed.foods.toLocaleString()} foods, {humanSize(
          status.installed_bytes,
        )}. {status.installed.attribution}
      </p>
      <div class="line">
        {#if armed}
          <button type="button" class="danger" onclick={remove}>
            Remove it and free {humanSize(status.installed_bytes)}
          </button>
          <button type="button" onclick={() => (armed = false)}>Keep it</button>
        {:else}
          <button type="button" onclick={() => (armed = true)}>Remove the food list</button>
        {/if}
      </div>
    {:else if progress?.running}
      <p class="muted" role="status">
        Getting the food list{part !== null ? ` — ${part}%` : '…'}
      </p>
      <div class="line"><button type="button" onclick={cancel}>Stop</button></div>
    {:else}
      <p class="muted">
        Search for a food and get its values and portions, with no connection needed afterwards.
        {humanSize(status.bytes)} to keep on this device. {status.licence}.
      </p>
      {#if !status.available}
        <p class="muted">It is not available to get yet. Foods can still be added by name.</p>
      {:else if progress && !progress.can_download}
        <p class="muted">This copy of the app cannot get it.</p>
      {:else}
        <div class="line">
          <button type="button" onclick={download}>Get the food list</button>
        </div>
      {/if}
    {/if}
  {/if}
  {#if error}<p class="muted err" role="alert">{error}</p>{/if}
</section>

<style>
  section {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }
  h4 {
    margin: var(--space-4) 0 0;
    font-size: var(--text-sm);
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
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .danger {
    color: var(--danger-fg);
  }
</style>

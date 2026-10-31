<script lang="ts">
  // A newer version, offered where people are rather than in Settings (`updater.svelte.ts`). One
  // press — Update — gets it, checks it and restarts into it; *Later* puts the line away for a few
  // days. Progress, a refusal and the restart are said in the same place the press was made.
  import * as updater from './updater.svelte';
  import { humanSize } from './size';
  import { phoneUpdatesInBrowser } from './ipc';

  const s = $derived(updater.status());
  const p = $derived(s?.progress ?? null);
  const v = $derived(s?.available ?? '');
  const phone = phoneUpdatesInBrowser();
</script>

{#if s && updater.offered()}
  <div class="update" role="status" aria-live="polite">
    {#if updater.restarting()}
      <span class="what">
        <strong>Restarting as {v}.</strong> This page reloads by itself in a moment.
      </span>
    {:else if updater.phoneMessage()}
      <span class="what">{updater.phoneMessage()}</span>
      <button type="button" onclick={updater.update}>Download again</button>
      <button type="button" onclick={updater.later}>Later</button>
    {:else if p && updater.inFlight(p)}
      <span class="what">
        <strong>Updating to {v}</strong> ·
        {updater.STAGE_WORDS[p.stage] ?? p.stage}…
        {#if p.stage === 'download'}
          {p.total ? `${humanSize(p.done)} of ${humanSize(p.total)}` : humanSize(p.done)}
        {/if}
      </span>
      <button type="button" onclick={updater.stop}>Stop</button>
    {:else if updater.going()}
      <span class="what"><strong>Updating to {v}</strong> · saving your work…</span>
    {:else if updater.error() || p?.stage === 'failed'}
      <span class="what">
        <strong>Could not update to {v}.</strong>
        {updater.error() ?? p?.error}
      </span>
      <button type="button" class="primary" onclick={updater.update}>Try again</button>
      <button type="button" onclick={updater.later}>Later</button>
    {:else}
      <span class="what">
        <strong>formicaria {v} is out</strong> — you have {s.current ?? 'an earlier version'}.
        {phone
          ? 'Your browser downloads it; your notes stay as they are.'
          : 'Your notes stay as they are; formicaria restarts by itself.'}
      </span>
      <button type="button" class="primary" onclick={updater.update}>Update</button>
      <button type="button" onclick={updater.later}>Later</button>
    {/if}
  </div>
{/if}

<style>
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    margin: 0;
    padding: var(--space-2) var(--space-5);
    font-size: var(--text-sm);
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--accent);
  }
  /* The top of the screen on a phone: clear the camera cutout, as App's own first banner does. */
  .update:first-child {
    padding-top: calc(var(--space-2) + var(--safe-top, 0px));
  }
  .what {
    flex: 1 1 16rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-contrast);
  }
</style>

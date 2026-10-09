<script lang="ts">
  // **Recently deleted** — `decisions.md` 2026-10-09, *a deleted note is never more than a list away*.
  //
  // Read from each notebook's own history (`deleted_notes`), so it survives closing the app and
  // reaches a deletion made on another device: git already kept every note's last text, and this is
  // the window onto it. **Bring back** writes the note again exactly as it was (`restore_deleted`),
  // refused if a note with that id exists again. A deletion from the last few seconds that has not
  // been saved yet is not here; the ↶ menu's Undo covers it.
  import { onMount } from 'svelte';
  import { deletedNotes, restoreDeleted, type DeletedRow } from './ipc';
  import { labelFor } from './vaults.svelte';

  let { onclose, onrestored }: { onclose: () => void; onrestored: () => void } = $props();

  let rows = $state<DeletedRow[] | null>(null);
  let error = $state('');
  let failed = $state<Record<string, string>>({});
  let busy = $state<string | null>(null);

  onMount(async () => {
    try {
      rows = await deletedNotes(30);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      rows = [];
    }
  });

  async function bringBack(r: DeletedRow) {
    busy = r.id;
    try {
      await restoreDeleted(r.vault, r.id);
      rows = (rows ?? []).filter((x) => x.id !== r.id);
      onrestored();
    } catch (e) {
      failed[r.id] = e instanceof Error ? e.message : String(e);
    } finally {
      busy = null;
    }
  }

  function when(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="deleted-overlay">
  <button class="deleted-backdrop" aria-label="close" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="recently deleted notes">
    <div class="panel-head">
      <h2>Recently deleted</h2>
      <button type="button" class="close" aria-label="close" onclick={onclose}>✕</button>
    </div>
    <p class="lede">
      Notes deleted in the last 30 days, on this device or another one that shares the notebook.
      Bringing one back puts it back exactly as it was.
    </p>

    {#if rows === null}
      <p class="lede">Looking…</p>
    {:else if error}
      <p class="failed" role="alert">{error}</p>
    {:else if rows.length === 0}
      <p class="lede">Nothing was deleted in the last 30 days.</p>
    {:else}
      <ul class="list">
        {#each rows as r (r.vault + r.id)}
          <li class="note">
            <div class="line">
              <div class="what">
                <div class="title">{r.title}</div>
                <div class="meta">
                  {labelFor(r.vault)} · {when(r.time)}{r.author ? ` · ${r.author}` : ''}
                </div>
              </div>
              <button type="button" disabled={busy !== null} onclick={() => bringBack(r)}>
                {busy === r.id ? 'Bringing back…' : 'Bring back'}
              </button>
            </div>
            {#if failed[r.id]}<p class="failed" role="alert">{failed[r.id]}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .deleted-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
    padding-bottom: var(--safe-bottom);
    z-index: 80;
  }
  @media (pointer: coarse) {
    .deleted-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .deleted-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(34rem, 92vw);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }
  .close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
  }
  .lede {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .note {
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .what {
    min-width: 0;
    flex: 1 1 12rem;
  }
  .title {
    color: var(--text);
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .meta {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .failed {
    margin: 4px 0 0;
    font-size: var(--text-sm);
    color: var(--danger, #b00);
  }
</style>

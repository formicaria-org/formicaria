<script lang="ts">
  // **Sync and backup, in the rail** (the shell, P2b; the parity list: the older frame's chips and its
  // Back up menu had no counterpart on the computer's pages). *Back up now* presses Back up for every
  // vault that has somewhere to send (`syncVault`: save, get the other devices' changes, send); a
  // chip appears only when there is something to look at — notes that could not be read, notes not
  // in history, notes the merge decided for you — and opens the same panel as the older frame.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import { busyLabel, syncFor, syncVault, syncingPhase } from '../lib/sync.svelte';
  import { labelFor } from '../lib/vaults.svelte';

  let { unsent = '' }: { unsent?: string } = $props();

  let skipped = $state<ipc.SkippedNote[]>([]);
  let unrecorded = $state<Awaited<ReturnType<typeof ipc.unrecorded>>>([]);
  let duplicates = $state<Awaited<ReturnType<typeof ipc.duplicates>>>([]);
  let demoted = $state<ipc.DemotedField[]>([]);
  let kept = $state<ipc.KeptNote[]>([]);
  let panel = $state<'' | 'skipped' | 'unrecorded' | 'kept'>('');
  let said = $state('');

  async function read() {
    const [p, u, d, dm, k] = await Promise.all([
      ipc.ping().catch(() => null),
      ipc.unrecorded().catch(() => []),
      ipc.duplicates().catch(() => []),
      ipc.demoted().catch(() => []),
      ipc.keptNotes().catch(() => []),
    ]);
    skipped = p?.skipped ?? [];
    unrecorded = u;
    duplicates = d;
    demoted = dm;
    kept = k;
  }
  onMount(() => {
    void read();
    const t = setInterval(() => void read(), 60_000);
    const stop = ipc.listenChanges(() => void read());
    return () => {
      clearInterval(t);
      stop();
    };
  });

  const notInHistory = $derived(unrecorded.reduce((n, u) => n + u.count, 0));
  const decided = $derived(
    new Set([...demoted.map((d) => d.id), ...kept.map((k) => k.id ?? k.path)]).size,
  );
  const busy = $derived(syncingPhase());

  async function backUpNow() {
    said = '';
    const st = await ipc.backupStatus().catch(() => null);
    const targets = (st?.vaults ?? []).filter((v) => !!v.remote).map((v) => v.name);
    if (!targets.length) {
      said = 'No vault has somewhere to send yet: Settings → Vaults & backup.';
      return;
    }
    const stamp = `${new Date().toISOString().slice(0, 16)}Z`;
    const results: string[] = [];
    for (const v of targets) {
      const phase = await syncVault(v, `backup: ${stamp}`);
      results.push(
        phase === 'synced'
          ? `${labelFor(v)}: sent`
          : `${labelFor(v)}: ${phase}${syncFor(v).error ? ` (${syncFor(v).error})` : ''}`,
      );
    }
    said = results.join(' · ');
    await read();
  }
  async function record(vault: string) {
    await ipc.recordUnrecorded(vault).catch((e) => (said = String(e)));
    await read();
  }
  async function prune(vault: string) {
    await ipc.pruneDuplicates(vault).catch((e) => (said = String(e)));
    await read();
  }
</script>

<div class="sync" aria-label="sync and backup" role="group">
  <button type="button" class="now" onclick={() => void backUpNow()} disabled={busy !== null}>
    {busy !== null ? busyLabel(busy) : 'Back up now'}
  </button>
  {#if unsent}<p class="chip warn" role="status">{unsent}</p>{/if}
  {#if skipped.length}
    <button type="button" class="chip warn" onclick={() => (panel = 'skipped')}
      >{skipped.length} unreadable</button
    >
  {/if}
  {#if notInHistory}
    <button type="button" class="chip" onclick={() => (panel = 'unrecorded')}
      >{notInHistory} not in history</button
    >
  {/if}
  {#if decided}
    <button type="button" class="chip" onclick={() => (panel = 'kept')}
      >{decided} decided for you</button
    >
  {/if}
  {#if said}<p class="said" role="status">{said}</p>{/if}
</div>

{#if panel === 'skipped'}
  {#await import('../lib/SkippedPanel.svelte') then { default: SkippedPanel }}
    <SkippedPanel {skipped} onclose={() => (panel = '')} />
  {/await}
{:else if panel === 'unrecorded'}
  {#await import('../lib/UnrecordedPanel.svelte') then { default: UnrecordedPanel }}
    <UnrecordedPanel
      {unrecorded}
      {duplicates}
      onrecord={record}
      onprune={prune}
      onclose={() => (panel = '')}
    />
  {/await}
{:else if panel === 'kept'}
  {#await import('../lib/KeptPanel.svelte') then { default: KeptPanel }}
    <KeptPanel rows={demoted} {kept} onclose={() => (panel = '')} onchanged={() => void read()} />
  {/await}
{/if}

<style>
  .sync {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
    margin-top: var(--space-2);
  }
  .now,
  .chip {
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
    margin: 0;
  }
  .warn {
    border-color: var(--warning-border, #d9a400);
    background: var(--warning-bg, #fff4d6);
  }
  .said {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
</style>

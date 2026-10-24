<script lang="ts">
  // The agenda's time zone, minimal (`zoneLabel.ts`): the app clock's city and offset (2026-10-24 —
  // no longer the browser's), with the notes' own clock in the tooltip. Recomputed every minute.
  import { onMount } from 'svelte';
  import { vaultList } from './vaults.svelte';
  import { zoneLabel } from './zoneLabel';
  import { appZone } from './appTime';

  let now = $state(new Date());
  onMount(() => {
    const t = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(t);
  });
  const label = $derived(
    zoneLabel(
      appZone(),
      (vaultList() ?? []).map((v) => v.clock),
      now,
    ),
  );
</script>

<span class="zone" title={label.title} data-testid="zone-label">{label.text}</span>

<style>
  .zone {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
    align-self: center;
    margin-left: var(--space-2);
  }
</style>

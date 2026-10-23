<script lang="ts">
  // The agenda's time zone, minimal (`zoneLabel.ts`): this device's city and offset, with the
  // notes' own clock in the tooltip. Recomputed every minute, so a device that changes zone shows it.
  import { onMount } from 'svelte';
  import { vaultList } from './vaults.svelte';
  import { deviceZone, zoneLabel } from './zoneLabel';

  let now = $state(new Date());
  onMount(() => {
    const t = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(t);
  });
  const label = $derived(
    zoneLabel(
      deviceZone(),
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

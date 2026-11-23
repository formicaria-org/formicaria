<script lang="ts">
  // The red dot beside Collaboration (`waiting.svelte.ts`). Drawn only when something waits.
  import { proposalsWaiting, waitingLabel } from './waiting.svelte';
  const count = $derived(proposalsWaiting());
</script>

{#if count > 0}
  <span class="waiting-dot" role="img" aria-label={waitingLabel(count)} title={waitingLabel(count)}
  ></span>
{/if}

<style>
  .waiting-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-left: 6px;
    border-radius: 50%;
    background: var(--u-overdue);
    vertical-align: middle;
    flex: none;
  }
</style>

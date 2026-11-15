<script lang="ts">
  // **formicaria's browser was updated while it was open** (0.33.1). The new copy waits beside the
  // running one and goes in at its next start, so say so where people are: one line, put away by
  // *Later* for this session. Looked up at start and every ten minutes; a few file reads.
  import { onMount } from 'svelte';
  import { browserStatus } from './ipc';

  let waiting = $state(false);
  let later = $state(false);

  async function look() {
    try {
      const st = await browserStatus();
      waiting = !!(st.supported && st.installed && st.update_ready && st.running);
    } catch {
      waiting = false;
    }
  }

  onMount(() => {
    look();
    const t = setInterval(look, 10 * 60 * 1000);
    return () => clearInterval(t);
  });
</script>

{#if waiting && !later}
  <div class="restart" role="status" aria-live="polite">
    <span class="what">
      <strong>formicaria’s browser has been updated.</strong> Close its window and open formicaria again
      to use the new version; your tabs come back.
    </span>
    <button type="button" onclick={() => (later = true)}>Later</button>
  </div>
{/if}

<style>
  .restart {
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
  .what {
    flex: 1 1 16rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 2.25rem;
    padding: 0 var(--space-3);
  }
</style>

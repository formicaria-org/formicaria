<script lang="ts">
  // A reminder that came due while the app is open: a line that stays until it is put away.
  import { putAway, showing } from './reminders.svelte';
</script>

{#if showing().length}
  <div class="reminders" role="status" aria-live="polite">
    {#each showing() as r (r.id)}
      <div class="reminder">
        <span class="what"><strong>{r.title}</strong> · {r.text}</span>
        <button type="button" onclick={() => putAway(r.id)} aria-label={`put away ${r.title}`}>
          OK
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .reminders {
    position: fixed;
    left: 50%;
    top: calc(var(--safe-top, 0px) + var(--space-3));
    transform: translateX(-50%);
    z-index: 90;
    width: min(30rem, calc(100% - 2 * var(--space-3)));
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .reminder {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
    box-shadow: var(--shadow-lg);
  }
  .what {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
</style>

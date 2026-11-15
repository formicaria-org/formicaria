<script lang="ts">
  // **What just came in from the browser, said in formicaria** (research §11.3, rules 2 and 3): one
  // line at the bottom, with **Undo** and the one alternative (*Link only instead*, *Keep the file*),
  // or *Open* for a saved page. Corrections after, rather than questions before. It goes by itself
  // after a few seconds, unless the pointer is on it.
  import { toast, undo, alternative } from './web/bridge.svelte';

  let { onopen }: { onopen: (id: string) => void } = $props();
  let hovered = $state(false);

  $effect(() => {
    const now = toast.now;
    if (!now || hovered) return;
    const t = setTimeout(() => {
      if (toast.now === now) toast.now = null;
    }, 8000);
    return () => clearTimeout(t);
  });
</script>

{#if toast.now}
  {@const t = toast.now}
  <div
    class="bring"
    role="status"
    aria-live="polite"
    onmouseenter={() => (hovered = true)}
    onmouseleave={() => (hovered = false)}
  >
    <span class="what">{t.text}</span>
    {#if t.alt}
      <button type="button" onclick={() => alternative(t)}>{t.alt.label}</button>
    {/if}
    {#if t.inserted}
      <button type="button" onclick={() => undo(t)}>Undo</button>
    {/if}
    {#if t.open}
      <button
        type="button"
        onclick={() => {
          onopen(t.open!);
          toast.now = null;
        }}>Open</button
      >
    {/if}
    <button type="button" class="x" aria-label="Close" onclick={() => (toast.now = null)}>×</button>
  </div>
{/if}

<style>
  .bring {
    position: fixed;
    left: 50%;
    bottom: calc(var(--space-4) + var(--safe-bottom, 0px));
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    max-width: min(36rem, calc(100vw - 2 * var(--space-4)));
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    background: var(--surface-elevated);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md, 8px);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.18);
  }
  .what {
    flex: 1 1 12rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 2rem;
    padding: 0 var(--space-2);
  }
  .x {
    min-width: 2rem;
  }
</style>

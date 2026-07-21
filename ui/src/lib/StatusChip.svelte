<script lang="ts">
  import { nextStatus } from './status';
  import { clickOutside } from './clickOutside';

  // Click to rotate a note's status through the values the vault already uses,
  // and through unset. No value is named here: `statuses` is learned from data
  // and the tint comes from [data-value] in the theme, so this chip renders any
  // workflow's statuses without a code change (same rule as the renderers).
  //
  // **Secondary gesture — a picker.** Rotate is the fast primary tap, but stepping to a distant
  // status is tedious with many of them, so a *secondary* gesture opens a menu to jump straight
  // there (or clear). `contextmenu` is that gesture on both platforms at once — right-click on a
  // laptop, long-press on a phone — leaving the primary tap untouched.
  let { status, statuses, onchange, title = 'Tap to rotate · long-press to pick' }: {
    status: string | null;
    statuses: string[];
    onchange: (next: string | null) => void;
    title?: string;
  } = $props();

  let picker = $state<{ top: number; left: number } | null>(null);

  function rotate(e: MouseEvent) {
    // A card is itself a click target that opens the note; rotating its status
    // must not also open it.
    e.stopPropagation();
    onchange(nextStatus(status, statuses));
  }

  function openPicker(e: MouseEvent) {
    e.preventDefault(); // no native context menu
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    picker = { top: r.bottom + 4, left: r.left };
  }

  function pick(e: MouseEvent, next: string | null) {
    e.stopPropagation();
    picker = null;
    onchange(next);
  }
</script>

<button
  class="status-chip"
  class:unset={!status}
  data-value={status ?? ''}
  onclick={rotate}
  oncontextmenu={openPicker}
  {title}
  aria-label={status ? `status: ${status}` : 'status: none'}
>
  {status ?? '+'}
</button>

{#if picker}
  <!-- The status picker: jump to any known status, or clear it. Anchored under the chip, dismissed
       by tapping outside. `statuses` is the vault's own set — data selects a value, never supplies one. -->
  <ul
    class="status-picker"
    role="listbox"
    aria-label="pick status"
    style="top: {picker.top}px; left: {picker.left}px"
    use:clickOutside={() => (picker = null)}
  >
    {#each statuses as s (s)}
      <li>
        <button class="status-opt" data-value={s} onclick={(e) => pick(e, s)}>{s}</button>
      </li>
    {/each}
    <li>
      <button class="status-opt none" onclick={(e) => pick(e, null)}>none</button>
    </li>
  </ul>
{/if}

<style>
  .status-chip {
    font: inherit;
    font-size: 0.72rem;
    line-height: 1.4;
    padding: 0.05rem 0.45rem;
    border-radius: 999px;
    border: 1px solid transparent;
    background: var(--tag-bg);
    color: var(--tag-fg);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .status-chip:hover {
    border-color: var(--border-strong);
  }
  /* No status yet: a quiet "+" affordance rather than an empty pill. */
  .status-chip.unset {
    background: transparent;
    border-color: var(--border);
    color: var(--muted);
  }
  /* The picker: a small menu anchored (fixed, to the viewport) under the chip. Each option is
     tinted by the same [data-value] rule the chip uses, so it previews the status colour. */
  .status-picker {
    position: fixed;
    z-index: 60;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    min-width: 7rem;
    max-width: min(14rem, 90vw);
    max-height: 60vh;
    overflow-y: auto;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  .status-opt {
    display: block;
    width: 100%;
    text-align: left;
    min-height: 2.25rem;
    padding: 0.1rem 0.5rem;
    margin: 1px 0;
    background: var(--tag-bg);
    color: var(--tag-fg);
    border: 1px solid transparent;
    border-radius: 999px;
    font: inherit;
    cursor: pointer;
  }
  .status-opt:hover {
    border-color: var(--border-strong);
  }
  .status-opt.none {
    background: transparent;
    color: var(--muted);
    border-color: var(--border);
  }
</style>

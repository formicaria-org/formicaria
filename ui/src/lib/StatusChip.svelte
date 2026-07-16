<script lang="ts">
  import { nextStatus } from './status';

  // Click to rotate a note's status through the values the vault already uses,
  // and through unset. No value is named here: `statuses` is learned from data
  // and the tint comes from [data-value] in the theme, so this chip renders any
  // workflow's statuses without a code change (same rule as the renderers).
  let { status, statuses, onchange, title = 'Click to change status' }: {
    status: string | null;
    statuses: string[];
    onchange: (next: string | null) => void;
    title?: string;
  } = $props();

  function rotate(e: MouseEvent) {
    // A card is itself a click target that opens the note; rotating its status
    // must not also open it.
    e.stopPropagation();
    onchange(nextStatus(status, statuses));
  }
</script>

<button
  class="status-chip"
  class:unset={!status}
  data-value={status ?? ''}
  onclick={rotate}
  {title}
  aria-label={status ? `status: ${status}` : 'status: none'}
>
  {status ?? '+'}
</button>

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
</style>

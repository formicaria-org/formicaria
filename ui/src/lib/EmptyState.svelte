<script lang="ts">
  import Icon from './Icon.svelte';

  // A calm empty/first-run state: an icon, a headline, an optional hint, and an
  // optional action. Replaces bare "Loading…" / "No assets yet." strings.
  let {
    icon = 'inbox',
    title,
    hint,
    actionLabel,
    onaction,
  }: {
    icon?: string;
    title: string;
    hint?: string;
    actionLabel?: string;
    onaction?: () => void;
  } = $props();
</script>

<div class="empty">
  <span class="icon"><Icon name={icon} size={32} /></span>
  <p class="title">{title}</p>
  {#if hint}<p class="hint">{hint}</p>{/if}
  {#if actionLabel && onaction}
    <button class="action" onclick={onaction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6);
    text-align: center;
  }
  .icon {
    color: var(--text-subtle);
    margin-bottom: var(--space-1);
  }
  .title {
    margin: 0;
    color: var(--text);
    font-size: var(--text-base);
    font-weight: 600;
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    max-width: 24rem;
  }
  .action {
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .action:hover {
    border-color: var(--accent);
  }
</style>

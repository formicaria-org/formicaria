<script lang="ts">
  // The shared "asset not found / not synced" placeholder. Media absence is a
  // warning, never an error: every surface that would show a blob (gallery tile,
  // card chip, read-view widget) falls back to this and keeps working. Used from
  // multiple renderers, so it lives in lib/, not under a single renderer.
  let { label }: { label?: string } = $props();
</script>

<div class="asset-missing" title={label ? `${label} — not available` : 'asset not available'}>
  <svg viewBox="0 0 24 24" width="26" height="26" aria-hidden="true">
    <rect
      x="3"
      y="4"
      width="18"
      height="16"
      rx="2"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
    />
    <circle cx="8.5" cy="9.5" r="1.6" fill="currentColor" />
    <path
      d="M4 17l4.5-4.5 3 3L15 12l5 5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linejoin="round"
    />
    <line x1="4" y1="20" x2="20" y2="4" stroke="currentColor" stroke-width="1.4" opacity="0.6" />
  </svg>
  <span>no preview</span>
</div>

<style>
  .asset-missing {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    color: var(--muted);
    font-size: 0.72rem;
    user-select: none;
  }
</style>

<script lang="ts">
  // The line a form shows when it opened on what was typed before and never saved (`drafts.ts`).
  let { onafresh }: { onafresh: () => void } = $props();
</script>

<p class="resumed" role="status">
  Carried on from where you left it.
  <button type="button" onclick={onafresh}>Start afresh</button>
</p>

<style>
  .resumed {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .resumed button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
</style>

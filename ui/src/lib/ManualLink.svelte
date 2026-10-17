<script lang="ts">
  /// **"How this works", beside the thing it explains** (`decisions.md` 2026-10-18). Opens the
  /// manual at the named section, in the reader, on every device. `ci/checks.sh` fails if `page`
  /// is not a page of the book or `section` is not a heading on it.
  import { openManual } from './manual.svelte';

  let {
    page,
    section,
    label = 'How this works',
  }: {
    /** The page, as the book names it: `user/appearance.html`. */
    page: string;
    /** The heading's id: `starting-from-a-suggestion`. */
    section?: string;
    label?: string;
  } = $props();
</script>

<button class="link manual-link" onclick={() => openManual(page, section)}>{label}</button>

<style>
  .manual-link {
    font-size: var(--text-sm);
  }
</style>

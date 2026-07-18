<script lang="ts">
  // "Who last touched this, and when" — shown on every card and the open note, straight from git
  // (via the reactive activity module). A person-coloured dot (matching their contributor filter
  // chip and activity rows) + their name + a relative time. Secondary to the vault badge, so it's
  // quiet: the dot carries the colour, the text stays readable. Renders nothing when git knows
  // nothing about the note (no history / single-user with no commits) — same "nothing to say,
  // show nothing" discipline as the vault badge.
  import { hashHue } from './vaultColor';
  import { relativeTime } from './stamp';
  import type { EditEvent } from './types';

  let { edit }: { edit?: EditEvent } = $props();

  // The placeholder committer signs a vault nobody else can see yet — that's "you", not a name.
  const name = $derived(edit ? (edit.email === 'formicaria@localhost' ? 'you' : edit.author) : '');
  const hue = $derived(name ? hashHue(name) : 0);
</script>

{#if edit && name}
  <span
    class="edited-by"
    style="--eh:{hue}"
    title={`Last edited by ${edit.author} · ${new Date(edit.time).toLocaleString()}`}
  >
    <span class="dot" aria-hidden="true"></span>
    <span class="who">{name}</span>
    <span class="when">· {relativeTime(edit.time)}</span>
  </span>
{/if}

<style>
  .edited-by {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    max-width: 12rem;
    font-size: 0.66rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .dot {
    flex: none;
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: hsl(var(--eh) 55% 48%);
    box-shadow: 0 0 0 1px hsl(var(--eh) 55% 32%);
  }
  .who {
    color: var(--text);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .when {
    flex: none;
  }
</style>

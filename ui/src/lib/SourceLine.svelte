<script lang="ts">
  // **A view says where its notes come from** (`docs/context/workspace-design-2026-11-15.md` §13.5):
  // one quiet line above a view whose notes a source brings (Mail's conversations: Gmail; what is
  // coming up: the calendars), with when it last read and a way to read now. Through the commands
  // Settings already uses; the line is absent where the source is not set up or not on this device.
  import { onMount } from 'svelte';
  import { calendarReadNow, calendars, mailReadNow, mailStatus } from './ipc';
  import { clockText } from './appTime';

  let { source }: { source: 'mail' | 'calendar' } = $props();
  let line = $state('');
  let busy = $state(false);

  async function look() {
    try {
      if (source === 'mail') {
        const m = await mailStatus();
        line =
          !m.configured || !m.connected
            ? ''
            : `Gmail · ${m.label}${m.lastChecked ? ` · read at ${clockText(m.lastChecked * 1000)}` : ''}`;
      } else {
        const c = await calendars();
        line = !c.feeds.length
          ? ''
          : `${c.feeds.length === 1 ? 'One calendar' : `${c.feeds.length} calendars`}${c.lastChecked ? ` · read at ${clockText(c.lastChecked * 1000)}` : ''}`;
      }
    } catch {
      line = '';
    }
  }
  async function now() {
    busy = true;
    try {
      if (source === 'mail') await mailReadNow();
      else await calendarReadNow();
    } catch {
      /* the line stays as it was */
    }
    busy = false;
    await look();
  }
  onMount(look);
</script>

{#if line}
  <p class="source-line" aria-label="where these notes come from">
    <span>{line}</span>
    <button type="button" onclick={now} disabled={busy}>{busy ? 'Reading…' : 'Read now'}</button>
  </p>
{/if}

<style>
  .source-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-muted, inherit);
  }
  .source-line span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 1.8rem;
    padding: 0 var(--space-2);
  }
</style>

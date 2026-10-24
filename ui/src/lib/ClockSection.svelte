<script lang="ts">
  /// **Settings → Clock** (`decisions.md#data`, 2026-10-24, *one app clock*): the zone every view
  /// shows and counts days in, on this device. Automatic follows the computer (or the phone); a
  /// chosen zone stays put whatever the computer says. Never synced: each device has its own.
  import { onMount } from 'svelte';
  import { clock, setClock } from './ipc';
  import { formatMoment, setAppClock, type ClockAnswer } from './appTime';

  let c = $state<ClockAnswer | null>(null);
  let choice = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let now = $state(Date.now());

  async function load() {
    try {
      c = await clock();
      choice = c.chosen ?? '';
      setAppClock(c);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function save(zone: string) {
    busy = true;
    error = null;
    try {
      c = await setClock(zone);
      choice = c.chosen ?? '';
      setAppClock(c);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void load();
    const id = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(id);
  });
</script>

<section data-cat="clock" data-words="time zone clock automatic computer phone">
  <h3>Time zone</h3>
  {#if c}
    <p class="line">
      <span>Now</span>
      <strong
        >{formatMoment(now, {
          weekday: 'short',
          day: 'numeric',
          month: 'short',
          hour: '2-digit',
          minute: '2-digit',
        })}</strong
      >
      <span class="muted">{c.zone ?? 'unknown'}{c.offset ? ` · ${c.offset}` : ''}</span>
    </p>
    <label class="line">
      <span>Zone</span>
      <input
        bind:value={choice}
        placeholder={c.system ? `Automatic — ${c.system} (this device's)` : 'Asia/Tokyo, or +09:00'}
        size="28"
        aria-label="time zone"
      />
      <button disabled={busy || choice.trim() === (c.chosen ?? '')} onclick={() => save(choice)}
        >Save</button
      >
      {#if c.chosen}
        <button disabled={busy} onclick={() => save('')}>Automatic</button>
      {/if}
    </label>
    <p class="muted small">
      {#if c.source === 'automatic'}
        Automatic: this device's own zone, {c.system ?? 'which cannot be read — choose one'}. When
        it changes, every view follows within a few minutes.
      {:else}
        Chosen here: times are shown on {c.chosen}, whatever zone this device is set to
        {#if c.system}(it is on {c.system}){/if}. Leave the field empty for Automatic.
      {/if}
      Every view, reminder and day — the agenda, food, health, the meetings found in mail — is on this
      clock. Each notebook keeps its own times, so the same meeting is at the right hour on every device.
    </p>
    {#if typeof c.skew_seconds === 'number' && Math.abs(c.skew_seconds) > 120}
      <p class="muted small warn" role="alert">
        This device's clock is {Math.round(Math.abs(c.skew_seconds) / 60)} minutes
        {c.skew_seconds > 0 ? 'ahead of' : 'behind'} the internet's ({c.skew_from}). Times and
        reminders follow this device's clock: set it right in the system's own settings.
      </p>
    {/if}
    {#if !c.zone}
      <p class="muted small warn" role="alert">
        This device's time zone cannot be read. Choose one above, or calendars and mail are not read
        in until you do.
      </p>
    {/if}
  {:else if !error}
    <p class="muted">Reading the clock…</p>
  {/if}
  {#if error}
    <p class="muted err" role="alert">{error}</p>
  {/if}
</section>

<style>
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin: var(--space-1) 0;
  }
  .line > span:first-child {
    min-width: 4em;
  }
  .small {
    font-size: var(--text-sm);
  }
  .warn {
    color: var(--warn, inherit);
  }
</style>

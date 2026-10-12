<script lang="ts">
  // Reminders: whether this device shows them (`decisions.md#ui`, 2026-10-13). Asked for, never
  // assumed: nothing reminds until the person turns it on here.
  import { isPhone } from './platform';
  import { allow, allowed, remindersOn, set } from './reminders.svelte';

  let can = $state(allowed());
  async function turnOn() {
    await allow();
    // The system's own question may still be on screen: look again shortly.
    can = allowed();
    setTimeout(() => (can = allowed()), 1500);
  }
</script>

<section>
  <h3>Reminders</h3>
  <label class="switch-line">
    <input
      type="checkbox"
      role="switch"
      checked={remindersOn()}
      onchange={(e) => ((e.currentTarget as HTMLInputElement).checked ? turnOn() : set(false))}
    />
    <span>Remind me of events and deadlines on this device</span>
  </label>
  <p class="muted">
    A note that is not finished is reminded of at its time, or at nine in the morning of its day
    when it has no time. Give a note the property <code>remind</code> with a number of minutes to be
    told earlier, or <code>off</code> to leave it out.
  </p>
  <p class="muted">
    {#if isPhone()}
      On this phone a reminder arrives even when the app is closed; it can be a few minutes late
      while the phone is asleep.
    {:else}
      On a computer a reminder arrives while formicaria is open in your browser.
    {/if}
    {#if remindersOn() && !can}
      The system is not allowing notifications for it yet, so it will show only as a line in the
      app. Allow them in your {isPhone() ? 'phone’s' : 'browser’s'} settings to change that.
    {/if}
  </p>
</section>

<style>
  .muted {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin: var(--space-2) 0 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .switch-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    cursor: pointer;
  }
</style>

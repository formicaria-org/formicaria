<script lang="ts">
  // Your own calendars, read into a notebook on their own — `decisions.md` 2026-10-09.
  //
  // The person pastes a calendar's private address once. After that this computer reads it every
  // hour, new meetings appear as notes, a rescheduled one moves, and a cancelled one is marked —
  // never deleted, and the note's own text is never touched. Nothing here asks for an approval,
  // because nothing here is a guess: an invite says exactly when it is.
  //
  // **The address never comes back from the server.** It is a secret — anyone holding it reads the
  // whole calendar — so the list shows only its host, and the paste field is a password field.
  import { onMount } from 'svelte';
  import {
    calendarAdd,
    calendarOffset,
    calendarReadNow,
    calendarRemove,
    calendars,
    type CalendarFeed,
    type Calendars,
  } from './ipc';
  import { labelFor, vaultList } from './vaults.svelte';

  let cal = $state<Calendars | null>(null);
  let error = $state('');
  let busy = $state(false);
  let label = $state('');
  let url = $state('');
  let vault = $state('');
  let offset = $state('');

  const vaults = $derived(vaultList() ?? []);

  /** The server answers a refusal as `{"error": "…"}`; show the sentence, not the JSON. */
  function sentence(e: unknown): string {
    const raw = e instanceof Error ? e.message : String(e);
    try {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.error === 'string') return parsed.error;
    } catch {
      /* not JSON — already a sentence */
    }
    return raw;
  }

  async function run(f: () => Promise<Calendars>) {
    busy = true;
    error = '';
    try {
      cal = await f();
      offset = cal.offset;
      return true;
    } catch (e) {
      error = sentence(e);
      return false;
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void run(calendars);
  });

  async function add() {
    if (await run(() => calendarAdd(label, url, vault))) {
      label = '';
      url = '';
      // Read it straight away, so the person sees their meetings without waiting for the hour.
      await run(calendarReadNow);
    }
  }

  function when(at: number): string {
    return at ? new Date(at * 1000).toLocaleString() : 'not yet';
  }

  /** One line per calendar, in counts — and the ones left out are said, not hidden. */
  function summary(f: CalendarFeed): string {
    const l = f.last;
    if (!l) return 'Not read yet.';
    if (l.error) return `Could not read it: ${l.error}`;
    const parts = [
      `${l.created ?? 0} new`,
      `${l.moved ?? 0} moved`,
      `${l.cancelled ?? 0} cancelled`,
    ];
    const left: string[] = [];
    if (l.past) left.push(`${l.past} already over`);
    if (l.repeating)
      left.push(
        `${l.repeating} repeating with a pattern formicaria cannot read yet (only the first date is shown)`,
      );
    if (l.undated) left.push(`${l.undated} with no date`);
    if (l.noId) left.push(`${l.noId} that could not be told apart`);
    const tail = left.length ? ` Left out: ${left.join(', ')}.` : '';
    // A repeating meeting is laid out week by week for the coming two months; saying so explains why
    // a series shows as many notes, and why later ones appear as time passes.
    const series = l.series
      ? ` ${l.series} repeating ${l.series === 1 ? 'meeting is' : 'meetings are'} shown for the next two months.`
      : '';
    return `${parts.join(', ')}.${series}${tail}`;
  }
</script>

<section data-testid="calendars">
  <h3>Calendars</h3>
  <p class="muted">
    Meetings from your own calendar, as notes. Paste the calendar's <strong>private address</strong>
    once — in Google Calendar it is under
    <em>Settings → your calendar → Integrate calendar → Secret address in iCal format</em>. This
    computer reads it every hour: new meetings appear, a moved one moves, a cancelled one is marked.
    Your own writing in a meeting note is never changed, and nothing is ever deleted. The address
    can only be read, never written to, and it stays on this computer — it is not saved in your
    notes.
  </p>

  {#if cal}
    <ul class="feeds">
      {#each cal.feeds as f (f.label)}
        <li>
          <div class="line">
            <strong>{f.label}</strong>
            <span class="muted small">{f.host} → {labelFor(f.vault)}</span>
            <button
              class="link"
              disabled={busy}
              aria-label={`Stop reading ${f.label}`}
              onclick={() => run(() => calendarRemove(f.label))}>Remove</button
            >
          </div>
          <p class="muted small" class:err={!!f.last?.error}>{summary(f)}</p>
        </li>
      {:else}
        <li class="muted">No calendars yet.</li>
      {/each}
    </ul>

    {#if cal.feeds.length}
      <div class="line">
        <button disabled={busy || cal.pulling} onclick={() => run(calendarReadNow)}>
          {busy || cal.pulling ? 'Reading…' : 'Read now'}
        </button>
        <span class="muted small">Last read {when(cal.lastChecked)}</span>
      </div>
    {/if}

    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <label>
        <span>Name</span>
        <input bind:value={label} placeholder="work" maxlength="40" />
      </label>
      <label>
        <span>Private address</span>
        <input
          type="password"
          bind:value={url}
          placeholder="https://calendar.google.com/…/basic.ics"
          autocomplete="off"
        />
      </label>
      <label>
        <span>Into</span>
        <select bind:value={vault}>
          {#each vaults as v (v.name)}
            <option value={v.name}>{labelFor(v.name)}</option>
          {/each}
        </select>
      </label>
      <button class="primary" type="submit" disabled={busy || !label.trim() || !url.trim()}>
        Add calendar
      </button>
    </form>

    <label class="line offset">
      <span>Your time zone</span>
      <input bind:value={offset} placeholder="+08:00" size="7" />
      <button
        disabled={busy || offset === cal.offset}
        onclick={() => run(() => calendarOffset(offset))}>Save</button
      >
    </label>
    <p class="muted small">
      As an offset from UTC, such as +08:00 for Singapore. Meetings are shown at this clock. A
      meeting written in another zone says which one on its note.
    </p>
  {/if}

  {#if error}
    <p class="muted err" role="alert">{error}</p>
  {/if}
</section>

<style>
  section {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }
  .muted {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin: var(--space-2) 0 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .small {
    font-size: 0.8rem;
  }
  .err {
    color: var(--danger, #b00020);
  }
  .feeds {
    list-style: none;
    padding: 0;
    margin: var(--space-2) 0;
  }
  .feeds li {
    padding: var(--space-2) 0;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .add {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
  .add label {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .add input,
  .add select {
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
  }
  .add span,
  .offset span {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .add button {
    justify-self: start;
  }
  button.link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    margin-left: auto;
  }
</style>

<script lang="ts">
  // Gmail, read-only — `decisions.md` 2026-10-09.
  //
  // Three steps, shown in order: save your own Google Cloud client, sign in with Google once, and
  // then nothing — mail with your label, since your date, is read every quarter of an hour. Each
  // conversation becomes one note (subject, last sender, when — never the text) and an invite inside
  // a message becomes a meeting note at once. The note holds the whole exchange, so the assistant
  // reads it like any other (`decisions.md`, *the email exchange lives in the conversation note*).
  //
  // **Read-only is Google's limit, not ours**: the one permission asked for cannot send, delete or
  // change anything, and the server refuses a sign-in that comes back with more.
  import { onDestroy, onMount } from 'svelte';
  import {
    mailConnect,
    mailDisconnect,
    mailReadNow,
    mailSetup,
    mailStatus,
    type MailStatus,
  } from './ipc';
  import { labelFor, vaultList } from './vaults.svelte';

  let st = $state<MailStatus | null>(null);
  let error = $state('');
  let busy = $state(false);
  let clientId = $state('');
  let clientSecret = $state('');
  let label = $state('formicaria');
  let since = $state('');
  let vault = $state('');
  let armed = $state(false);
  let poll: ReturnType<typeof setInterval> | null = null;

  const vaults = $derived(vaultList() ?? []);

  /** Two weeks ago, as YYYY-MM-DD — the suggested start. */
  function twoWeeksAgo(): string {
    const d = new Date(Date.now() - 14 * 86400_000);
    return d.toISOString().slice(0, 10);
  }

  function sentence(e: unknown): string {
    const raw = e instanceof Error ? e.message : String(e);
    try {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.error === 'string') return parsed.error;
    } catch {
      /* already a sentence */
    }
    return raw;
  }

  async function run(f: () => Promise<MailStatus>) {
    busy = true;
    error = '';
    try {
      st = await f();
      return true;
    } catch (e) {
      error = sentence(e);
      return false;
    } finally {
      busy = false;
    }
  }

  function adopt(s: MailStatus) {
    label = s.label || 'formicaria';
    since = s.since || twoWeeksAgo();
    vault = s.vault;
  }

  onMount(async () => {
    if (await run(mailStatus)) adopt(st!);
  });
  onDestroy(() => {
    if (poll) clearInterval(poll);
  });

  async function save() {
    if (await run(() => mailSetup({ clientId, clientSecret, label, since, vault }))) {
      clientId = '';
      clientSecret = '';
    }
  }

  /** Open Google's page in a new tab **synchronously**, then point it at the consent address once the
   *  server has made one — a tab opened after an `await` is what a popup blocker stops. */
  async function signIn() {
    const tab = window.open('', '_blank');
    busy = true;
    error = '';
    try {
      const { url } = await mailConnect(location.origin);
      if (tab) tab.location.href = url;
      else location.href = url;
      // Watch for the sign-in to land, for up to five minutes.
      let left = 100;
      poll = setInterval(async () => {
        left -= 1;
        try {
          st = await mailStatus();
        } catch {
          /* the next tick tries again */
        }
        if (st?.connected || left <= 0) {
          if (poll) clearInterval(poll);
          poll = null;
        }
      }, 3000);
    } catch (e) {
      tab?.close();
      error = sentence(e);
    } finally {
      busy = false;
    }
  }

  function when(at: number): string {
    return at ? new Date(at * 1000).toLocaleString() : 'not yet';
  }

  function summary(s: MailStatus): string {
    const l = s.last;
    if (!l) return 'Not read yet.';
    if (l.error) return `Could not read it: ${l.error}`;
    const parts = [
      `${l.messages ?? 0} new ${l.messages === 1 ? 'message' : 'messages'}`,
      `${l.newConversations ?? 0} new conversations`,
      `${l.updatedConversations ?? 0} with replies`,
    ];
    const inv = l.invites;
    const meetings =
      inv && inv.created + inv.moved + inv.cancelled
        ? ` Invites: ${inv.created} new, ${inv.moved} moved, ${inv.cancelled} cancelled.`
        : '';
    const more =
      l.waiting && l.waiting >= 200 ? ' More are waiting and will be read next time.' : '';
    return `${parts.join(', ')}.${meetings}${more}`;
  }
</script>

<section data-testid="mail">
  <h3>Mail</h3>
  <p class="muted">
    Read Gmail to keep your meetings and conversations up to date. formicaria can <strong
      >only read</strong
    >
    — Google gives it no way to send, delete or change mail — and it reads only mail with
    <strong>your label</strong>, from <strong>the date you choose</strong>. Each conversation
    becomes one note holding the whole exchange — who wrote to whom, when, and what they said. The
    exchange goes wherever that notebook goes: if it is copied online, so is your mail. An
    invitation in a message becomes a meeting straight away.
  </p>

  {#if st}
    {#if st.connected}
      <p class="line">
        <strong>Connected</strong>
        <span class="muted small">{st.email}</span>
      </p>
      <p class="muted small" class:err={!!st.last?.error}>{summary(st)}</p>
      <div class="line">
        <button disabled={busy || st.reading} onclick={() => run(mailReadNow)}>
          {busy || st.reading ? 'Reading…' : 'Read now'}
        </button>
        <span class="muted small">Last read {when(st.lastChecked)} · {st.read} messages so far</span
        >
      </div>
    {:else if st.configured}
      <p class="muted small" class:err={!!st.last?.error}>
        {st.last?.error ?? 'Ready to sign in.'}
      </p>
      <button class="primary" disabled={busy} onclick={signIn}>Sign in with Google</button>
      <p class="muted small">
        Google shows a warning that it has not checked this app — that is because the app is your
        own Google Cloud project. Choose <em>Continue</em>. While the project is in testing, Google
        asks you to sign in again every 7 days; this panel will say when.
      </p>
    {/if}

    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      {#if !st.configured}
        <p class="muted small">
          First, your own Google Cloud client (a one-time set-up — the manual's <em>Mail</em> page walks
          through it). Paste its two values here.
        </p>
      {/if}
      <label>
        <span>Client ID</span>
        <input
          bind:value={clientId}
          placeholder={st.configured
            ? 'saved — paste only to replace it'
            : '…apps.googleusercontent.com'}
          autocomplete="off"
        />
      </label>
      <label>
        <span>Client secret</span>
        <input
          type="password"
          bind:value={clientSecret}
          placeholder={st.configured ? 'saved — paste only to replace it' : 'GOCSPX-…'}
          autocomplete="off"
        />
      </label>
      <label>
        <span>Gmail label to read</span>
        <input bind:value={label} placeholder="formicaria" />
      </label>
      <label>
        <span>Read mail from</span>
        <input type="date" bind:value={since} />
      </label>
      <label>
        <span>Into</span>
        <select bind:value={vault}>
          {#each vaults as v (v.name)}
            <option value={v.name}>{labelFor(v.name)}</option>
          {/each}
        </select>
      </label>
      {#if !label.trim()}
        <p class="muted small err">
          With no label, every message since that date is read — consider a label instead.
        </p>
      {/if}
      <button
        type="submit"
        disabled={busy || (!st.configured && (!clientId.trim() || !clientSecret.trim()))}
      >
        Save
      </button>
    </form>

    {#if st.connected}
      <div class="line">
        {#if armed}
          <button
            class="danger"
            disabled={busy}
            onclick={() => run(mailDisconnect).then(() => (armed = false))}
          >
            Disconnect Gmail
          </button>
          <button onclick={() => (armed = false)}>Keep it</button>
        {:else}
          <button class="link" onclick={() => (armed = true)}>Disconnect…</button>
        {/if}
      </div>
      {#if armed}
        <p class="muted small">
          Stops all reading and asks Google to cancel the sign-in. The notes already made stay.
        </p>
      {/if}
    {/if}
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
  .add span {
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
  }
</style>

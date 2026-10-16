<script lang="ts">
  // **What the assistant made of your mail**, in Collaboration (2026-10-16). A reply that only
  // confirms a meeting proposes nothing, and with nothing to show the owner took it for the mail never
  // having been read ("[a collaborator] replied … still see no change"). So each conversation the assistant read
  // gets one line — "nothing to change — the meeting stays at …" is an answer too — beside when the
  // mail was last checked.
  //
  // Asks on its own every 15 s while the page is visible: a verdict changes no note, so the vault's
  // own change beat never fires for it. Shows nothing where there is no mail (the phone, the mock).
  import { mailStatus, mailConnect, type MailStatus } from './ipc';
  import { MAIL_LINKS } from './mailLinks';

  let { onopen }: { onopen: (id: string) => void } = $props();

  let st = $state<MailStatus | null>(null);

  async function ask() {
    try {
      st = await mailStatus();
    } catch {
      st = null;
    }
  }

  $effect(() => {
    void ask();
    const id = setInterval(() => {
      if (!document.hidden) void ask();
    }, 15000);
    const onVisible = () => {
      if (!document.hidden) void ask();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
    };
  });

  /** **A stopped reader is said here, with its remedy** (2026-10-16). Google ends a sign-in every 7
   *  days for a project in testing; the reason was only in Settings, and this section hid itself
   *  while disconnected — so mail simply stopped arriving, and the owner saw nothing change for 20
   *  minutes. Opens Google's page in a tab made *before* the await, which a popup blocker allows. */
  let signingIn = $state(false);
  let signInError = $state('');
  async function signIn() {
    const tab = window.open('', '_blank');
    signingIn = true;
    signInError = '';
    try {
      const { url } = await mailConnect(location.origin);
      if (tab) tab.location.href = url;
      else location.href = url;
    } catch (e) {
      tab?.close();
      signInError = e instanceof Error ? e.message : String(e);
    } finally {
      signingIn = false;
    }
  }

  /** "2 min ago", "15:04", or a date — short enough for one line on a phone. */
  function ago(at: number): string {
    if (!at) return 'not yet';
    const secs = Math.max(0, Date.now() / 1000 - at);
    if (secs < 60) return 'just now';
    if (secs < 3600) return `${Math.round(secs / 60)} min ago`;
    const d = new Date(at * 1000);
    return d.toDateString() === new Date().toDateString()
      ? d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      : d.toLocaleDateString();
  }
</script>

{#if st?.configured && !st.connected}
  <section class="mail-heard stopped" role="alert">
    <h3 class="mail-head">Mail <span class="err">stopped</span></h3>
    <p class="heard-said">
      {st.last?.error ?? 'Gmail is not signed in, so no mail is being read.'} Nothing reaches your notes
      until you sign in again.
    </p>
    <button class="sign-in" type="button" disabled={signingIn} onclick={signIn}>
      {signingIn ? 'Opening Google…' : 'Sign in with Google'}
    </button>
    {#if signInError}<p class="heard-said err">{signInError}</p>{/if}
    <p class="heard-said">
      Signed out after 7 days? That is Google's rule while your Google Cloud project is in testing —
      <a href={MAIL_LINKS.AUDIENCE} target="_blank" rel="noopener noreferrer">publish the app</a>
      to stop it (<a href={MAIL_LINKS.MANUAL_WEEKLY} target="_blank" rel="noopener noreferrer"
        >how, step by step</a
      >).
    </p>
  </section>
{:else if st?.connected}
  <section class="mail-heard">
    <h3 class="mail-head">
      Mail
      <span class="mail-checked">
        {st.reading ? 'checking now…' : `checked ${ago(st.lastChecked)}`}
        {#if st.last?.error}· <span class="err">{st.last.error}</span>{/if}
      </span>
    </h3>
    {#if st.heard?.length}
      <ul>
        {#each st.heard as h (h.note)}
          <li>
            <button class="heard-row" onclick={() => onopen(h.note)}>
              <span class="heard-title">{h.title}</span>
              <span class="heard-when">{ago(h.at)}</span>
            </button>
            <p class="heard-said">
              Read up to the email of {h.latest.replace('T', ' ')}: {h.said}
            </p>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="heard-said">The assistant has not read a conversation yet.</p>
    {/if}
  </section>
{/if}

<style>
  .mail-heard {
    margin: var(--space-2);
  }
  .mail-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .mail-checked {
    font-weight: normal;
    font-size: var(--text-xs, 0.75rem);
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .err {
    color: var(--danger, #c33);
  }
  .stopped {
    padding: var(--space-2);
    border: 1px solid var(--danger, #c33);
    border-radius: var(--radius-md);
  }
  .sign-in {
    margin: var(--space-2) var(--space-2) 0;
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent, #fff);
    font: inherit;
    cursor: pointer;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    border-bottom: 1px solid var(--border, rgba(127, 127, 127, 0.18));
    padding-bottom: var(--space-1);
  }
  .heard-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-2) 0;
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .heard-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .heard-when {
    flex-shrink: 0;
    font-size: var(--text-xs, 0.75rem);
    color: var(--muted);
  }
  .heard-said {
    margin: 0;
    padding: 0 var(--space-2);
    font-size: var(--text-sm);
    color: var(--muted);
    overflow-wrap: anywhere;
  }
</style>

<script lang="ts">
  // **The first screen a tablet sees**, and for an unpaired device the only one.
  //
  // It stands in for the whole app rather than appearing as a banner over it, because a 401
  // means *every* command failed, not one of them: an app rendered behind this would be an empty
  // board, an empty agenda and an error on each — a broken-looking product where the truth is
  // simply "you have not been let in yet".
  //
  // Nothing here ever touches the token. The code buys an `HttpOnly` cookie, which no script can
  // read; a device that pairs successfully just reloads into an app that now works. That matters
  // because note bodies arrive from collaborators, so the page is not a place to keep a
  // credential.
  import { pairDevice, clearPairingFlag } from './ipc';

  let code = $state('');
  let name = $state(defaultName());
  let busy = $state(false);
  let error = $state('');
  let vaults = $state<string[] | null>(null);

  // A name for this device, so the person at the computer can tell which tablet they just let
  // in. Guessed rather than demanded — it is editable, and a wrong guess costs nothing.
  function defaultName(): string {
    const ua = navigator.userAgent;
    if (/iPad/.test(ua)) return 'iPad';
    if (/iPhone/.test(ua)) return 'iPhone';
    if (/Android/.test(ua)) return /Mobile/.test(ua) ? 'Android phone' : 'Android tablet';
    if (/Macintosh/.test(ua)) return 'Mac';
    if (/Windows/.test(ua)) return 'Windows PC';
    return 'This device';
  }

  async function submit(e: Event) {
    e.preventDefault();
    if (busy || code.trim().length === 0) return;
    busy = true;
    error = '';
    try {
      const r = await pairDevice(code, name);
      vaults = r.vaults;
      clearPairingFlag();
      // Reload rather than continue in place: every query this page ran before pairing failed,
      // and re-running them by hand from here would mean teaching this component about all of
      // them. A reload is one line and cannot miss one.
      setTimeout(() => globalThis.location.reload(), 900);
    } catch (err) {
      // The server's text is the message — it distinguishes a typo from an expired code from a
      // code that was *cancelled* because someone kept guessing, and those need different
      // reactions from the person holding the tablet.
      error = String(err instanceof Error ? err.message : err);
      code = '';
    } finally {
      busy = false;
    }
  }
</script>

<div class="pair">
  <div class="card">
    {#if vaults}
      <h1>Paired</h1>
      <p class="ok">
        This device can now open
        {vaults.length === 1 ? vaults[0] : `${vaults.length} vaults`}. Opening…
      </p>
    {:else}
      <h1>Connect to your notes</h1>
      <p class="lede">
        On the computer sharing this vault, open <strong
          >Settings → Share with another device</strong
        > and start a code. Then type it here.
      </p>

      <form onsubmit={submit}>
        <label for="pair-code">Pairing code</label>
        <!-- `autocapitalize` and the uppercase transform because the alphabet is uppercase and a
             tablet keyboard will not be, by default. `inputmode=text` keeps the letter keys up. -->
        <input
          id="pair-code"
          bind:value={code}
          placeholder="ABCD2345"
          autocomplete="off"
          autocapitalize="characters"
          spellcheck="false"
          maxlength="12"
          disabled={busy}
        />

        <label for="pair-name">Name this device</label>
        <p class="hint">So you can recognise it on the computer.</p>
        <input id="pair-name" bind:value={name} autocomplete="off" disabled={busy} />

        <button type="submit" disabled={busy || !code.trim()}>
          {busy ? 'Connecting…' : 'Connect'}
        </button>
      </form>

      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}

      <p class="foot">
        Codes last a few minutes and work once. Your notes stay on the computer — this device reads
        and writes them over your own network, and nothing is sent anywhere else.
      </p>
    {/if}
  </div>
</div>

<style>
  /* Sized for a tablet held in the hand: one column, large targets, no chrome. Everything is
     `var(--…)` so it follows the app's own theme rather than inventing a second palette. */
  .pair {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: var(--bg);
    color: var(--fg);
  }
  .card {
    width: min(28rem, 100%);
  }
  h1 {
    font-size: 1.5rem;
    margin: 0 0 0.5rem;
  }
  .lede,
  .foot,
  .hint {
    color: var(--fg-dim);
    line-height: 1.5;
  }
  .lede {
    margin: 0 0 1.5rem;
  }
  .hint {
    font-size: 0.85rem;
    margin: 0 0 0.35rem;
  }
  .foot {
    font-size: 0.85rem;
    margin-top: 2rem;
  }
  label {
    display: block;
    font-weight: 600;
    margin-top: 1rem;
  }
  input {
    width: 100%;
    box-sizing: border-box;
    /* 16px or larger, or iOS Safari zooms the page on focus. */
    font-size: 1.05rem;
    padding: 0.7rem 0.8rem;
    margin-top: 0.35rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-elev);
    color: var(--fg);
  }
  #pair-code {
    font-family: var(--mono, monospace);
    letter-spacing: 0.2em;
    text-transform: uppercase;
  }
  button {
    width: 100%;
    margin-top: 1.25rem;
    padding: 0.8rem;
    font-size: 1rem;
    font-weight: 600;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-fg, #fff);
  }
  button:disabled {
    opacity: 0.5;
  }
  .error {
    margin-top: 1rem;
    color: var(--danger, #c0392b);
  }
  .ok {
    color: var(--fg-dim);
  }
</style>

<script lang="ts">
  /// **The one question git will otherwise ask at the worst possible moment.**
  ///
  /// Git refuses to commit anything without a committer. The app covers for that — `ensure_repo`
  /// writes a placeholder identity, so a notebook works perfectly with nobody's name on it — but a
  /// placeholder is permanent in a way a setting is not: it is stamped into every commit, and git
  /// history does not get corrected later. The moment it starts to matter is the moment someone
  /// else clones the vault, and by then it is months of commits too late.
  ///
  /// So it is asked once, at the start, where the answer is cheap and nothing has been recorded
  /// yet — and it is **skippable**, because a notebook that demands a name before it will hold a
  /// note has misunderstood what it is. Someone who skips keeps full history under the placeholder
  /// and is asked again by `BackupPanel` at the moment a remote makes a name matter.
  ///
  /// # What this screen is not
  ///
  /// Not a setup wizard, and deliberately not the vault form. The archive now ships a ready vault
  /// with a welcome note in it, so `NewVault`'s first-run branch no longer fires for most people —
  /// this is what greets them instead, and it must stay short enough to read in one breath. Three
  /// fields, two of them optional, and a way past it.
  ///
  /// **It does not explain the app**, and must not start to: dismissing this lands the reader on a
  /// Timeline whose one note is `packaging/welcome/`, which is where "what is this and where do I
  /// click" is answered. Two screens, two jobs — this one asks the single question git forces.
  ///
  /// **Hidden entirely where git is absent** — see the gate in `App.svelte`. A machine with no git
  /// has no committer to name and no remote to push to, so every field here would be a control
  /// that cannot do anything: the notebook works, and saying so is `vaultCheck`'s job, not a
  /// form's.

  import { setIdentity, setGitRemote } from './ipc';

  interface Props {
    /// The vault this names a committer for — the default one, which is where every fresh note
    /// lands. `''` means "the default", which is what every command here already understands.
    vault?: string;
    /// Dismissed: either saved, or skipped. The caller decides what to remember.
    ondone: () => void;
  }
  let { vault = '', ondone }: Props = $props();

  let name = $state('');
  let email = $state('');
  let remote = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  /// The identity landed but the remote did not. **Both halves are reported**, because "saved" and
  /// "failed" are each half true and printing only one of them is how someone retypes a name that
  /// is already stored, or walks away believing a remote is set.
  let identitySaved = $state(false);

  const canSave = $derived(!busy && !!name.trim() && !!email.trim());

  async function save() {
    if (!canSave) return;
    busy = true;
    error = null;
    try {
      // **Identity first, and on its own.** `setGitRemote` can carry an identity, but only
      // alongside a URL — and these two fail for unrelated reasons: a typo'd remote must not cost
      // you the name you just typed correctly. So they are two calls in a fixed order, and a
      // failure of the second leaves the first standing and says so.
      await setIdentity(name.trim(), email.trim(), vault);
      identitySaved = true;
      const url = remote.trim();
      if (url) await setGitRemote(url, undefined, vault);
      ondone();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<main class="welcome">
  <div class="box">
    <h1>Welcome to formicaria</h1>
    <p class="lede">
      Your notes are ordinary files on this computer, and they are already safe. Two details make
      their history useful — and you can add them later instead.
    </p>

    <label>
      <span>Your name</span>
      <input bind:value={name} placeholder="Ada Lovelace" autocomplete="name" disabled={busy} />
    </label>

    <label>
      <span>Your email</span>
      <input
        bind:value={email}
        placeholder="ada@example.org"
        autocomplete="email"
        spellcheck="false"
        disabled={busy}
      />
      <small>
        Every saved change is stamped with these, so you can tell who wrote what. Nothing is sent
        anywhere.
      </small>
    </label>

    <label>
      <span>A backup repository <em>(optional)</em></span>
      <input
        bind:value={remote}
        placeholder="git@github.com:you/notes.git"
        spellcheck="false"
        disabled={busy}
        onkeydown={(e) => e.key === 'Enter' && void save()}
      />
      <small>
        Somewhere off this computer for your notes to go. You can set this up whenever you like,
        under <strong>Back up</strong>.
      </small>
    </label>

    {#if error}
      <!-- Both halves, always. The identity is stored the instant its call returns, so an error
           from the *remote* must not read as "nothing was saved". -->
      <p class="error" role="alert">
        {#if identitySaved}Your name was saved. The backup repository was not:{/if}
        {error}
      </p>
    {/if}

    <div class="actions">
      <!-- Skipping is a real answer and looks like one. A notebook that demands a name before it
           will hold a note has misunderstood what it is; the placeholder identity keeps history
           working, and BackupPanel asks again when a remote makes a name matter. -->
      <button type="button" class="ghost" onclick={ondone} disabled={busy}>Skip for now</button>
      <button type="button" onclick={() => void save()} disabled={!canSave}>
        {busy ? 'Saving…' : 'Start writing'}
      </button>
    </div>
  </div>
</main>

<style>
  .welcome {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1.5rem;
    background: var(--bg);
    color: var(--text);
  }
  .box {
    width: 100%;
    max-width: 26rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  h1 {
    margin: 0;
    font-size: 1.4rem;
    font-weight: 600;
  }
  .lede {
    margin: 0;
    font-size: 0.95rem;
    color: var(--text-dim, inherit);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.9rem;
  }
  label em {
    font-style: normal;
    opacity: 0.6;
  }
  input {
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border, currentColor);
    border-radius: 0.4rem;
    background: transparent;
    color: inherit;
    font: inherit;
  }
  small {
    font-size: 0.8rem;
    opacity: 0.75;
  }
  .error {
    margin: 0;
    color: var(--danger, crimson);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }
  button {
    /* Its own size rule, not `.icon-btn`: those are hidden in the narrow layouts, which is exactly
       how a control that must stay reachable on a phone disappears there. */
    min-height: 2.75rem;
    padding: 0 1.25rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border, currentColor);
    background: var(--surface, transparent);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .ghost {
    border-color: transparent;
    background: transparent;
    opacity: 0.8;
  }
</style>

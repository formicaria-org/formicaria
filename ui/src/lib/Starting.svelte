<script lang="ts">
  /// **The screen that exists so there is never a blank one.**
  ///
  /// The app's render gate has always had three states — `null` (not asked yet), `[]` (first run),
  /// a list (the app) — and `null` deliberately rendered *nothing*, on the grounds that flashing a
  /// first-run form at someone with ten vaults is a lie we would tell for 40 ms. That reasoning is
  /// right and is kept: this component shows nothing for the first {@link DELAY_MS}.
  ///
  /// What it fixes is the case nobody had a screen for: `null` that never resolves. On Android the
  /// webview is created *before* the shell has opened the vaults, so the first `list_vaults` can be
  /// answered late — or, when startup failed, never. The user then holds a phone showing a blank
  /// window with no error, no spinner and nothing to tap, which is exactly how "the app is broken"
  /// and "the app is still opening" became indistinguishable (the owner's phone, 2026-07-31: a gray
  /// screen on the first open, fine on the second).
  ///
  /// So: silence, then a status, then — if the backend gave a reason — the reason and a Retry.
  /// Deliberately dependency-free and layout-trivial: this is the screen that has to work when
  /// nothing else does.

  interface Props {
    /// The message the backend last refused with, if any. `null` while we are simply waiting —
    /// which includes the case that matters most, a call that never answers at all.
    error?: string | null;
    /// How many times the boot call has been tried, so a retry that keeps failing can say so
    /// rather than looking like the first attempt forever.
    attempts?: number;
    /// How long we have been waiting, in ms. **This is what makes a permanent failure stop reading
    /// as a transient one** — the first version said "Opening your vaults…" identically at one
    /// second and at three minutes, which is the same "no information" the blank screen had.
    waitedMs?: number;
    onretry: () => void;
  }
  let { error = null, attempts = 1, waitedMs = 0, onretry }: Props = $props();

  /// Long enough that a normal launch never shows this at all (the desktop answers in single-digit
  /// milliseconds), short enough that a phone user is not left guessing.
  const DELAY_MS = 700;
  /// When waiting stops being normal. A cold phone launch opens every vault and rebuilds the search
  /// index, so several seconds is honest; past this it is worth saying so, and worth offering the
  /// button. Before it, a button with nothing wrong to fix invites a tap that starts a second boot.
  const SLOW_MS = 8000;
  let visible = $state(false);
  $effect(() => {
    const id = setTimeout(() => (visible = true), DELAY_MS);
    return () => clearTimeout(id);
  });
  const slow = $derived(waitedMs >= SLOW_MS);
  /// The shell's own "not yet" sentence is a *status*, not a fault, and printing it under a heading
  /// that says the same thing was the screen telling the user one thing twice.
  const transient = $derived(!!error && /still opening your vaults/i.test(error));
  const fault = $derived(error && !transient ? error : null);
</script>

{#if visible}
  <main class="starting" aria-live="polite">
    <div class="box">
      <p class="status">
        {#if fault}
          formicaria could not open your vaults.
        {:else if slow}
          Still opening your vaults — this is taking longer than it should.
        {:else}
          Opening your vaults…
        {/if}
      </p>
      {#if fault}
        <!-- The backend's own sentence, verbatim. On a phone this is the only diagnostic channel
             there is: stdout is not routed to logcat and the WebView forwards no console output
             (see docs/context/known-issues.md), so anything the user must be able to report has
             to be on screen. -->
        <p class="why">{fault}</p>
      {/if}
      {#if (fault || slow) && attempts > 1}
        <p class="hint">Tried {attempts} times.</p>
      {/if}
      {#if fault || slow}
        <button type="button" onclick={onretry}>Try again</button>
      {/if}
    </div>
  </main>
{/if}

<style>
  .starting {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1.5rem;
    background: var(--bg);
    color: var(--text);
  }
  .box {
    max-width: 28rem;
    text-align: center;
  }
  .status {
    margin: 0 0 0.75rem;
    font-size: 1.05rem;
  }
  .why {
    margin: 0 0 0.75rem;
    color: var(--text-dim, inherit);
    /* A backend error can be a long path or a git message; it must wrap rather than push the
       viewport sideways on a narrow screen. */
    overflow-wrap: anywhere;
    font-size: 0.9rem;
  }
  .hint {
    margin: 0 0 0.75rem;
    font-size: 0.85rem;
    opacity: 0.7;
  }
  button {
    /* Its own size rule rather than `.icon-btn`: every `.icon-btn` in the top bar is hidden in the
       narrow layouts, which is precisely how a control that must stay visible on a phone vanishes
       there (known-issues.md). Both axes set, for the same reason a round button needs both. */
    min-height: 2.75rem;
    padding: 0 1.25rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border, currentColor);
    background: var(--surface, transparent);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
</style>

<script lang="ts">
  /// **The manual, inside the app, the same on every device** (`decisions.md` 2026-10-18). A page of
  /// the book in a frame, over whatever you were doing; Done, Escape or Back closes it and you are
  /// where you were.
  ///
  /// **`sandbox="allow-scripts"` is the point, not a detail.** The book needs its inline scripts
  /// (chapter list, search, its own dark/light), so it runs under a looser policy than the app. In
  /// a sandboxed frame without `allow-same-origin` it has an opaque origin: it cannot reach this
  /// document, its storage or the app's commands — on a computer, where it is served from the same
  /// server, as much as on a phone, where it comes from its own scheme.
  ///
  /// Above Settings (z-index 80), because Settings is where most of its links are.
  import { closeManual, manualUrl, openPage } from './manual.svelte';

  const url = $derived(openPage());
</script>

{#if url}
  <div class="sheet-backdrop" role="presentation" onclick={closeManual}></div>
  <div class="sheet">
    <div
      class="reader"
      role="dialog"
      aria-modal="true"
      aria-label="The manual"
      tabindex="-1"
      onkeydown={(e) => e.key === 'Escape' && closeManual()}
    >
      <div class="bar">
        <strong>Manual</strong>
        <span class="spacer"></span>
        <a class="link" href={manualUrl()} target="manual-frame">Contents</a>
        <button class="done" onclick={closeManual}>Done</button>
      </div>
      <iframe
        name="manual-frame"
        title="The manual"
        src={url}
        sandbox="allow-scripts"
        referrerpolicy="no-referrer"
      ></iframe>
    </div>
  </div>
{/if}

<style>
  /* The same overlay as Help (`HelpPanel.svelte` says why it is copied, not shared): bounded by the
     visible screen and its insets, never past it. */
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 90;
  }
  .sheet {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    height: var(--app-h);
    padding: var(--safe-top) var(--safe-right) var(--safe-bottom) var(--safe-left);
    z-index: 91;
    pointer-events: none;
  }
  @media (pointer: coarse) {
    .sheet {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .reader {
    pointer-events: auto;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    width: min(60rem, 96vw);
    height: 100%;
    max-height: min(56rem, 100%);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .spacer {
    flex: 1;
  }
  .link {
    color: var(--link);
    font-size: var(--text-sm);
  }
  .done {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  iframe {
    flex: 1;
    width: 100%;
    min-height: 0;
    border: 0;
  }
</style>

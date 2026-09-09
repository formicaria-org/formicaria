<script lang="ts">
  /// **Help that exists on every device.**
  ///
  /// The manual is baked into `fm-serve` and served at `/manual/`. On the phone the UI comes from
  /// the Tauri shell instead, so that path resolves to nothing — and rather than ship a button that
  /// 404s, the Help button was hidden there entirely. The result was a phone with no help at all,
  /// in an app whose first non-technical tester's verdict on Help was that it was "difficult to
  /// find and click on".
  ///
  /// So this is the floor: a short page, in the bundle, on every device. It is not the manual and
  /// does not try to be — where the full book is reachable it says so and links to it. Embedding
  /// mdBook in the mobile shell is a real piece of work (its own protocol scheme and its own CSP,
  /// exactly as `MANUAL_CSP` already exists for on the desktop); this is what makes the gap
  /// survivable in the meantime, and it stays useful afterwards as the one-screen version.
  import { isPhone } from './platform';

  interface Props {
    onclose: () => void;
  }
  let { onclose }: Props = $props();
</script>

<div class="sheet-backdrop" role="presentation" onclick={onclose}></div>
<div
  class="sheet"
  role="dialog"
  aria-modal="true"
  aria-label="How this works"
  tabindex="-1"
  onkeydown={(e) => e.key === 'Escape' && onclose()}
>
  <div class="help">
    <h2>How this works</h2>

    <h3>Writing something</h3>
    <p>
      Press <strong>+</strong> and choose <strong>New note</strong>. There is no folder to pick and
      no title to invent first. To change a note later, <strong>double-click</strong> it. While
      editing, typing <strong>/</strong> offers to link another note or drop in a file.
    </p>

    <h3>Finding it again</h3>
    <p>
      Your notes are not filed in one place. You look at the same notes in different ways, and the
      strip of buttons at the side of the window lists them — pick one. On a narrow window it is the <strong
        >Views</strong
      > button.
    </p>
    <ul>
      <li><strong>Timeline</strong> — everything, by the day you wrote it.</li>
      <li>
        <strong>Board</strong> — cards in columns. Drag a card; the column is just a property.
      </li>
      <li><strong>Agenda</strong> — a calendar and a list, for anything you gave a date.</li>
      <li><strong>Search</strong> — a word anywhere in any note.</li>
    </ul>

    <h3>Giving a note a shape</h3>
    <p>
      Double-click a note to edit it, then use the <strong>+</strong> in its header to add a title, tags,
      a status or dates. A status puts it on the Board; a date puts it on the Agenda. None of it is required.
    </p>

    <h3>Keeping it safe</h3>
    <p>
      <strong>Back up</strong> records the history of your notes and can send a copy somewhere you own.
      Photos, PDFs and recordings are larger, so they are a separate, optional step in the same place.
    </p>

    <!-- **The assistant belongs here, and on the phone this is the only place it can be.** Android
         has no manual link (the button below is desktop-only), so without this the Settings row is
         the sole text about a feature that is bundled in every phone build. Short, and it names the
         two things that are not guessable: how to call it, and that it is off until you say so. -->
    <h3>Asking the assistant</h3>
    <p>
      Optionally, a small AI model runs <strong>on this device</strong> — your notes never leave it.
      Turn it on in <strong>Settings → Study assistant</strong>; the first time, it asks which model
      and how much to download. Then, in any note's discussion, type <code>@</code> and pick it:
      <code>@qwen3-vl-4b summarise this</code>. Add <code>/search</code> to let it look something up
      first, or <code>/propose</code> to have it draft an edit you approve before anything changes. It
      never edits a note on its own.
    </p>

    <h3>Changing how it looks</h3>
    <p>
      <strong>Settings → Appearance</strong> sets the accent colour, text size and font, and can hold
      a theme of your own. If a theme ever makes the app unusable, close it and open it again — a theme
      you could not click through is not switched on a second time.
    </p>

    {#if !isPhone()}
      <p class="more">
        <button
          class="link"
          onclick={() => {
            window.open('/manual/', '_blank', 'noopener');
            onclose();
          }}>Open the full manual</button
        >
        — every feature, offline, in this app.
      </p>
    {/if}

    <div class="actions">
      <button class="done" onclick={onclose}>Done</button>
    </div>
  </div>
</div>

<style>
  /* **These two rules were missing entirely, and Help did not work.** The markup below was
     copied from `App.svelte`'s `.sheet` dialogs when Help was extracted into its own
     component, but the CSS was not: Svelte scopes `.sheet` to App's own elements, and
     `.sheet > :global(*)` globalises the *child* selector, not `.sheet` itself. So these
     divs got no `position: fixed`, no backdrop and no centring, and Help rendered as an
     in-flow block inside `.app` — which is `overflow: hidden`. Verified in the built CSS:
     `.sheet` was emitted only as `.sheet.svelte-<App's hash>`.

     Copying the block, rather than reaching for `:global(.sheet)`, is deliberate — a global
     rule here would tie on specificity with App's scoped one and be decided by bundle
     order, which is the defect `ci/checks.sh` already polices for `.panel-views`. */
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 40;
  }
  .sheet {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    height: 100dvh;
    padding: var(--safe-top) var(--safe-right) var(--safe-bottom) var(--safe-left);
    z-index: 41;
    pointer-events: none;
  }
  /* The navigation-bar floor — `--bar-floor`, never a hand-copied number. */
  @media (pointer: coarse) {
    .sheet {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .help {
    pointer-events: auto;
    box-sizing: border-box;
    width: min(34rem, 92vw);
    max-width: 34rem;
    /* `100%` of `.sheet`'s content box — never `80vh`; see `--overlay-inset` in `app.css`. */
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--space-4);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-3, 10px);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-lg);
  }
  h3 {
    margin: var(--space-4) 0 var(--space-1);
    font-size: var(--text-base);
    color: var(--text);
  }
  p,
  li {
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--lh-base);
  }
  p {
    margin: 0 0 var(--space-2);
  }
  ul {
    margin: 0 0 var(--space-2);
    padding-left: var(--space-4);
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--link);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
  .more {
    margin-top: var(--space-4);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-4);
  }
  .done {
    font: inherit;
    padding: var(--space-2) var(--space-4);
    min-height: 2.75rem;
    background: var(--accent);
    color: var(--accent-contrast);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
</style>

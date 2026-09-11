<script lang="ts">
  // The open views, and the controls of the one you are looking at.
  //
  // Only meaningful in the `single` arrangement, where one pane fills the screen and the others
  // are hidden rather than unmounted — so switching is instant and this bar is pure navigation,
  // never a fetch trigger.
  //
  // **Wide, a tab per open window; narrow, the one you are in and a counted button for the rest**
  // (2026-09-11, `decisions.md#ui`). On a phone a tab per window spent the top row on what you are
  // *not* looking at — three windows were three tabs and a close button across the whole width. What
  // the owner asked for is what a phone browser does: one square carrying the number of open
  // windows, with the list of them behind it. Both are rendered and the width decides which shows,
  // in CSS, so there is no viewport-tracking TypeScript here either.
  import Icon from './Icon.svelte';
  import ViewControls from './ViewControls.svelte';
  import { paneTitle, BUILTIN_PANES, type Pane, type Feed } from './panes';

  let {
    panes,
    active,
    feed,
    onselect,
    onchange,
    onclose,
  }: {
    panes: Pane[];
    active: number;
    /// The active pane's fetched data — `ViewControls` needs it to say what a saved view hides.
    feed: Feed | undefined;
    onselect: (i: number) => void;
    onchange: (patch: Partial<Pane>) => void;
    onclose: (id: string) => void;
  } = $props();

  // The pane kinds map onto icons we already ship; anything without one (a saved view) falls back
  // to a dot rather than inventing art. Built-in icons come from the one registry, so a new kind
  // added there is iconed here automatically; `note` is added on top (it is not a built-in pane).
  const ICONS: Record<string, string> = {
    ...Object.fromEntries(BUILTIN_PANES.map((b) => [b.kind, b.icon])),
    note: 'pen',
  };

  /// The list behind the counted button, open only while you are choosing.
  let listOpen = $state(false);

  function choose(i: number) {
    listOpen = false;
    onselect(i);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (listOpen && e.key === 'Escape') listOpen = false;
  }}
/>

<nav class="viewbar" aria-label="open views">
  {#each panes as pane, i (pane.id)}
    <button
      class="tab"
      class:active={i === active}
      aria-current={i === active ? 'page' : undefined}
      onclick={() => onselect(i)}
      title={paneTitle(pane)}
    >
      {#if ICONS[pane.kind]}
        <Icon name={ICONS[pane.kind]} size={18} />
      {:else}
        <span class="dot" aria-hidden="true">•</span>
      {/if}
      <span class="label">{paneTitle(pane)}</span>
    </button>
  {/each}

  <!-- **The active view's own controls**, at the far end of the row that names it. This is the
       whole reason the pane header could go: grouping, density, a search pane's query box and a
       saved view's rename/delete are about the view in front of you, and this row is already
       about the view in front of you. One row instead of two. -->
  {#if panes[active]}
    <div class="controls">
      <ViewControls pane={panes[active]} {feed} {onchange} />
    </div>
  {/if}

  <!-- **No Settings gear here.** It lived in this bar back when the bar was the phone's only way
       into anything that is not a view. The chrome bar carries it now on every screen, and two
       buttons with one name is something two test files had to work around. -->

  <!-- Closing is offered here as well as in the pane header: this strip is where you can see
       *which* window you are closing, next to the others. On a narrow screen it lives in the list
       behind the counted button instead, beside each window's name. -->
  {#if panes.length > 1}
    <button
      class="tab close"
      onclick={() => onclose(panes[active].id)}
      aria-label="close this view"
      title="Close this view"
    >
      <Icon name="close" size={16} />
    </button>
  {/if}

  <!-- **The counted button**, shown on a narrow screen in place of the other tabs and the close
       button. The number is every open window including this one, the way a phone browser counts
       its tabs. -->
  <button
    class="windows"
    aria-label="open windows: {panes.length}"
    aria-expanded={listOpen}
    title="Open windows"
    onclick={() => (listOpen = !listOpen)}
  >
    <span class="count" aria-hidden="true">{panes.length}</span>
  </button>
</nav>

{#if listOpen}
  <button
    class="windows-backdrop"
    aria-label="close the list of windows"
    onclick={() => (listOpen = false)}
  ></button>
  <div class="windows-list" role="dialog" aria-label="open windows">
    <ul>
      {#each panes as pane, i (pane.id)}
        <li class:active={i === active}>
          <button
            class="pick"
            aria-current={i === active ? 'true' : undefined}
            onclick={() => choose(i)}
          >
            {#if ICONS[pane.kind]}
              <Icon name={ICONS[pane.kind]} size={18} />
            {:else}
              <span class="dot" aria-hidden="true">•</span>
            {/if}
            <span class="pick-label">{paneTitle(pane)}</span>
          </button>
          {#if panes.length > 1}
            <button
              class="shut"
              aria-label="close {paneTitle(pane)}"
              title="Close this window"
              onclick={() => onclose(pane.id)}
            >
              <Icon name="close" size={16} />
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .viewbar {
    /* Mounted only when one view fills the window (`App.svelte` branches on the layout
       preference), so there is nothing left for CSS to decide here. */
    display: flex;
    /* `.body` is a column flex container and `.workspace` takes the rest — without this the bar
       would shrink under a tall board instead of the board scrolling. */
    flex: 0 0 auto;
    align-items: stretch;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--surface);
    /* **A top bar since 2026-08-31.** The border and the inset swap ends with it: `--safe-top` is
       what puts this below a camera cutout, and on the owner's phone that is 52px — the shell
       reports the real number now rather than the 28px the stylesheet used to guess. */
    border-bottom: 1px solid var(--border);
    padding-top: var(--safe-top);
    padding-left: max(var(--safe-left), 0px);
    padding-right: max(var(--safe-right), 0px);
  }
  .viewbar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    flex: 1 0 auto;
    min-width: 4.5rem;
    /* Sized for the pointer that is actually on it. A wide touch tablet still shows every tab, and
       that is a fact about input, not about platform — the coarse-pointer rule below restores 3rem. */
    min-height: 2.25rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: var(--space-1) var(--space-2);
    background: transparent;
    border: none;
    border-top: 2px solid transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .tab.active {
    color: var(--text);
    border-top-color: var(--accent);
    background: var(--surface-hover);
  }
  /* **The spacer is gone.** It was there to push the utilities to the far end, but `.tab`'s own
     `flex: 1 0 auto` already does that — so with one or two windows open it was simply a wide
     band of empty grey, which is what it looked like. */
  .tab.close {
    flex: 0 0 auto;
    min-width: 3rem;
  }
  /* The window you are in is named in full; the others are just enough to recognise. */
  .tab.active .label {
    max-width: 12rem;
    font-weight: 600;
  }
  .label {
    max-width: 6rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    font-size: 1rem;
    line-height: 1;
  }
  /* Pushed to the far end and allowed to shrink before the tabs do — the tabs are how you get
     anywhere, the controls only tune where you already are. */
  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 0 1 auto;
    min-width: 0;
    margin-inline-start: auto;
    padding-inline: var(--space-2);
  }

  /* The counted button: not shown where there is room for a tab per window. */
  .windows {
    display: none;
    flex: 0 0 auto;
    min-width: 3rem;
    place-items: center;
    padding: 0 var(--space-2);
    background: transparent;
    border: none;
    color: var(--text);
    cursor: pointer;
  }
  .count {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    min-width: 1.5rem;
    height: 1.5rem;
    padding: 0 0.2rem;
    border: 2px solid currentColor;
    border-radius: 0.4rem;
    font-size: var(--text-xs);
    font-weight: 700;
    line-height: 1;
  }
  .windows-backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    padding: 0;
    background: transparent;
    border: none;
  }
  .windows-list {
    position: fixed;
    z-index: 61;
    /* Just below this bar, which is the safe-area inset plus a 3rem tab. */
    top: calc(var(--safe-top) + 3.5rem);
    right: max(var(--safe-right), var(--space-2));
    width: min(20rem, calc(100vw - 2 * var(--space-3)));
    max-height: 60dvh;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  .windows-list ul {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
  }
  .windows-list li {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: var(--radius-sm);
  }
  .windows-list li.active {
    background: var(--surface-hover);
  }
  .pick {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .pick-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .windows-list li.active .pick-label {
    font-weight: 600;
  }
  .shut {
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    min-width: 2.75rem;
    min-height: 2.75rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  /* A wide touch tablet shows every tab and has no mouse — the honest test is the pointer, not
     the width. 2.75rem is the ~44px both platform guidelines ask for. */
  @media (pointer: coarse) {
    .tab {
      min-height: 3rem;
    }
  }
  /* **A phone: the window you are in, its controls, and the counted button.** The same breakpoint
     as `App.svelte`'s, where the app already decides it is on a phone; last in the sheet, per the
     2026-09-08 width ruling. `.tab.close` is a `.tab` that is never active, so it goes too. */
  @media (max-width: 40rem) {
    .tab:not(.active) {
      display: none;
    }
    .windows {
      display: grid;
    }
  }
</style>

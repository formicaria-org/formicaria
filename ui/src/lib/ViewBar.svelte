<script lang="ts">
  // The open views, and the controls of the one you are looking at.
  //
  // Only meaningful in the `single` arrangement, where one pane fills the screen and the others
  // are hidden rather than unmounted — so switching is instant and this bar is pure navigation,
  // never a fetch trigger.
  //
  // **Not on a phone** (2026-09-11, `decisions.md#ui`). Below 40rem this row is hidden: the owner
  // asked for the top of the phone back, since a row naming the view in front of you told them
  // nothing they could not see. What it carried moved rather than vanished — the count and list of
  // open windows are a button in the bottom bar beside the view button, and this view's own switches
  // (Month/Week/List, Feed/List, a search pane's query, a filtered view's chip) head the view menu.
  // Wider screens keep a tab per window here. CSS decides, so there is no viewport-tracking
  // TypeScript.
  import Icon from './Icon.svelte';
  import ViewControls from './ViewControls.svelte';
  import { paneTitle, paneIcon, type Pane, type Feed } from './panes';

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
</script>

<nav class="viewbar" aria-label="open views">
  {#each panes as pane, i (pane.id)}
    {@const icon = paneIcon(pane)}
    <button
      class="tab"
      class:active={i === active}
      aria-current={i === active ? 'page' : undefined}
      onclick={() => onselect(i)}
      title={paneTitle(pane)}
    >
      {#if icon}
        <Icon name={icon} size={18} />
      {:else}
        <span class="dot" aria-hidden="true">•</span>
      {/if}
      <span class="label">{paneTitle(pane)}</span>
    </button>
  {/each}

  <!-- **The active view's own controls**, at the far end of the row that names it. This is the
       whole reason the pane header could go: grouping, density, a search pane's query box and a
       saved view's rename/delete are about the view in front of you, and this row is already
       about the view in front of you. One row instead of two. On a phone the same controls head
       the view menu instead. -->
  {#if panes[active]}
    <div class="controls">
      <ViewControls pane={panes[active]} {feed} {onchange} />
    </div>
  {/if}

  <!-- **No Settings gear here.** It lived in this bar back when the bar was the phone's only way
       into anything that is not a view. The chrome bar carries it now on every screen, and two
       buttons with one name is something two test files had to work around. -->

  <!-- Closing is offered here as well as in the pane header: this strip is where you can see
       *which* window you are closing, next to the others. -->
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
</nav>

<style>
  .viewbar {
    /* Mounted only when one view fills the window (`App.svelte` branches on the layout
       preference), so there is nothing left for CSS to decide here — except the width below. */
    display: flex;
    /* `.body` is a column flex container and `.workspace` takes the rest — without this the bar
       would shrink under a tall board instead of the board scrolling. */
    flex: 0 0 auto;
    align-items: stretch;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--surface);
    /* **A top bar since 2026-08-31.** `--safe-top` is what puts this below a camera cutout. On a
       phone, where this row is hidden, `.body` pays that inset instead (`App.svelte`). */
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
    /* Sized for the pointer that is actually on it. A wide touch tablet still shows this row, and
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
  /* A wide touch tablet shows this strip and has no mouse — the honest test is the pointer, not
     the width. 2.75rem is the ~44px both platform guidelines ask for. */
  @media (pointer: coarse) {
    .tab {
      min-height: 3rem;
    }
  }
  /* **Not on a phone.** The same breakpoint as `App.svelte`'s, where the app decides it is on a
     phone; last in the sheet, per the 2026-09-08 width ruling. */
  @media (max-width: 40rem) {
    .viewbar {
      display: none;
    }
  }
</style>

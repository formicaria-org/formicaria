<script lang="ts">
  // The open views, as somewhere your thumb can actually reach.
  //
  // Only meaningful in the `single` arrangement, where one pane fills the screen and the others
  // are hidden rather than unmounted — so switching is instant and this bar is pure navigation,
  // never a fetch trigger.
  //
  // **It is no longer on the phone** (2026-08-31). It was a tab per open window plus a count plus
  // a dead spacer, stacked above the bottom bar — about a third of a phone screen spent on what
  // you are *not* looking at. On a small screen the answer is the active view and its name; the
  // list of everything open is a desktop affordance, where there is width to spare and a pointer
  // to use it.
  //
  // Rendered always and shown by CSS (the arrangement's own tokens in `App.svelte`), so
  // there is no conditional component tree — the same discipline the rest of the layout follows.
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
</script>

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
    /* Sized for the pointer that is actually on it. This strip is desktop-only by width now, so
       a thumb target is wasted height — but a wide touch tablet still shows it, and that is a
       fact about input, not about platform. The coarse-pointer rule below restores 3rem. */
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
</style>

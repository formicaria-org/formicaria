<script lang="ts">
  // The open views, as somewhere your thumb can actually reach.
  //
  // Only meaningful in the `single` arrangement, where one pane fills the screen and the others
  // are hidden rather than unmounted — so switching is instant and this bar is pure navigation,
  // never a fetch trigger.
  //
  // **At the bottom on purpose.** On a 2712px-tall phone the top of the screen is not reachable
  // one-handed, which is why every mobile platform put primary navigation at the bottom and left
  // it there. It is also where a rail or drawer grows from if this ever needs a tablet layout.
  //
  // Rendered always and hidden by CSS in `tiled`, so there is no conditional component tree —
  // the same discipline the rest of the layout follows.
  import Icon from './Icon.svelte';
  import { paneTitle, BUILTIN_PANES, type Pane } from './panes';

  let {
    panes,
    active,
    onselect,
    onclose,
    onsettings,
  }: {
    panes: Pane[];
    active: number;
    onselect: (i: number) => void;
    onclose: (id: string) => void;
    /** Settings — the one utility, and on a phone the way into everything that is not a view. */
    onsettings: () => void;
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
      title={paneTitle(pane)}>
      {#if ICONS[pane.kind]}
        <Icon name={ICONS[pane.kind]} size={18} />
      {:else}
        <span class="dot" aria-hidden="true">•</span>
      {/if}
      <span class="label">{paneTitle(pane)}</span>
    </button>
  {/each}

  <!-- **The palette and Settings live down here, not in the top bar.**
       Wrapping the top bar stopped it hiding it, but on a phone it bought that with a second
       cramped row above the content. This bar is already the navigation, already thumb-reachable,
       and has room — so the one control that is not *navigation* sits at its end, pushed right so
       it never moves as views are opened and closed. -->
  <div class="spacer" aria-hidden="true"></div>
  <button class="tab util" onclick={onsettings} aria-label="settings" title="Settings">
    <Icon name="gear" size={18} />
    <span class="label">Settings</span>
  </button>

  <!-- Closing lives here because in `single` the pane's own header chrome is hidden: with one
       view filling the screen there is no room for a title bar, and no ambiguity about which
       pane a close button means. -->
  {#if panes.length > 1}
    <button
      class="tab close"
      onclick={() => onclose(panes[active].id)}
      aria-label="close this view"
      title="Close this view">
      <Icon name="close" size={16} />
    </button>
  {/if}
</nav>

<style>
  .viewbar {
    display: none; /* shown only by the single-pane rules in App.svelte */
    align-items: stretch;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--surface);
    border-top: 1px solid var(--border);
    /* Clear the gesture pill / home indicator. Zero where there is none, so a desktop window
       showing this layout is unaffected. */
    padding-bottom: var(--safe-bottom);
    padding-left: max(var(--safe-left), 0px);
    padding-right: max(var(--safe-right), 0px);
  }
  .viewbar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    flex: 1 0 auto;
    min-width: 4.5rem;
    /* 2.75rem is the ~44px both platform guidelines ask for; this row is the one place a
       mis-tap costs you the view you were reading. */
    min-height: 3rem;
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
  /* Pushes the utilities to the far end, so they hold one position while the view tabs to
     their left come and go. */
  .spacer {
    flex: 1 0 auto;
    min-width: var(--space-2);
  }
  .tab.util {
    flex: 0 0 auto;
    min-width: 3.5rem;
    border-top-color: transparent;
  }
  .tab.close {
    flex: 0 0 auto;
    min-width: 3rem;
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
</style>

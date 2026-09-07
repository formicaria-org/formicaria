<script lang="ts">
  /// **What belongs to the view you are looking at** — its name, the one knob that tunes it, and
  /// (for a saved view) what it leaves out. **Not how to rename or remove it**: that went with the
  /// authoring surface on 2026-08-31, as the paragraph below says. This line said otherwise until
  /// 2026-09-05, which is one file asserting two opposite things about itself.
  ///
  /// Extracted from `Pane.svelte`'s header on 2026-08-31 so it can be rendered in **two places
  /// from one definition**: at the right-hand end of the top view bar when one view fills the
  /// window, and in the pane header when several are tiled. Two half-copies drifting apart is
  /// exactly how the old command palette became a second Settings — `decisions.md` says so, and
  /// `targetsFrom` in `App.svelte` is the same discipline applied to the view list.
  ///
  /// **Rendered by exactly one caller at a time.** `App.svelte` branches on `workspace.layout`,
  /// not on the viewport — a preference string, the same one `data-layout` already carries, so the
  /// "no viewport-tracking TypeScript" rule is untouched. It has to be a branch rather than CSS:
  /// two copies in the DOM means two elements labelled `group by`, which breaks assistive
  /// navigation and makes `getByLabelText` ambiguous.
  ///
  /// **No view *authoring* here — only view *modes*** (2026-08-31). Save, Rename, Delete and the
  /// board's group-by box are gone: *"views are basically fixed for now and view customization will
  /// need its own design plan."* The commands survive in `fm-app` and `ipc.ts`, and a `.view` file
  /// still lists and opens, so this is a withdrawn surface rather than a deleted feature.
  ///
  /// What is left is the distinction that survived the cut. Month/Week/List and Feed/List do not
  /// re-fetch anything — `feedKey` is a constant for both kinds — they choose which component
  /// draws the cards already in hand. They are two ways of reading one view, not an edit to it.
  /// The search box does re-key its fetch, and is the only place the query a pane is *showing* can
  /// be seen or amended; the chrome's field can replace it but never reads it back.
  import { rendererKind, type Pane, type Feed } from './panes';

  interface Props {
    pane: Pane;
    feed: Feed | undefined;
    onchange: (patch: Partial<Pane>) => void;
  }
  let { pane, feed, onchange }: Props = $props();

  /// **No name in here.** Both callers already name the view next to this: the bar's tabs label
  /// every open view and mark the active one, and a pane header carries its own label. Printing it
  /// a third time is the duplication this component exists to end.

  /// **What the saved view on show leaves out**, read from the *feed* rather than from the view
  /// list: the words belong to the payload they describe and arrive with the board itself. Looking
  /// them up in `list_views` would make the explanation depend on a second fetch that runs once per
  /// vault change and has failed outright on the phone — and a filtered board with no explanation
  /// is precisely the bug this exists for (2026-08-24).
  const shownView = $derived(pane.kind === 'view' ? feed?.view : undefined);
  const hides = $derived(shownView?.filters ?? []);
  /// The whole sentence, including the way out — the visible chip is one line and gets clipped in a
  /// narrow pane, so the accessible name has to be the complete thought on its own.
  const hidesTitle = $derived(
    `“${pane.viewName}” shows only notes where ${hides.join(', and ')}. Click to see everything.`,
  );
  /// Leave the view for the built-in renderer it shadows, unfiltered, keeping its grouping — the
  /// one click that answers "where did my column go".
  function showEverything() {
    if (!shownView) return;
    onchange({
      kind: rendererKind(shownView.renderer),
      viewName: null,
      groupBy: shownView.group_by ?? pane.groupBy,
    });
  }
</script>

{#if pane.kind === 'agenda'}
  <div class="seg">
    <button class:on={pane.agendaMode === 'month'} onclick={() => onchange({ agendaMode: 'month' })}
      >M</button
    >
    <button class:on={pane.agendaMode === 'week'} onclick={() => onchange({ agendaMode: 'week' })}
      >W</button
    >
    <button class:on={pane.agendaMode === 'list'} onclick={() => onchange({ agendaMode: 'list' })}
      >L</button
    >
  </div>
{:else if pane.kind === 'timeline'}
  <!-- One renderer, two densities, chosen per window and remembered. Words rather than initials —
       there are only two, and "Feed"/"List" say what they are without being learned. -->
  <div class="seg">
    <button
      class:on={pane.timelineMode !== 'compact'}
      onclick={() => onchange({ timelineMode: 'feed' })}
      title="Each note as a post, with its picture">Feed</button
    >
    <button
      class:on={pane.timelineMode === 'compact'}
      onclick={() => onchange({ timelineMode: 'compact' })}
      title="One line per note">List</button
    >
  </div>
{:else if pane.kind === 'view' && hides.length}
  <!-- **A filtered view has to admit it.** Everything else here tunes a view; this one explains
       it. The words come from the server (`ViewInfo.filters`), so nothing here knows what a status
       is and a view narrowing by tag or date reads just as well. The text is *visible*, not tucked
       into `title`: a phone has no hover, and the point is to be legible at the moment the column
       looks missing. -->
  <button
    type="button"
    class="hides"
    onclick={showEverything}
    title={hidesTitle}
    aria-label={hidesTitle}
  >
    filtered: {hides.join(' · ')}
  </button>
{:else if pane.kind === 'search'}
  <!-- **Not redundant with the chrome's search field.** That one retargets this pane, but its own
       box is never seeded from `pane.query` — so without this you could replace the search and
       never see or amend what you are currently searching for. -->
  <input
    class="ctl"
    type="search"
    placeholder="Search…"
    value={pane.query}
    oninput={(e) => onchange({ query: (e.currentTarget as HTMLInputElement).value })}
    spellcheck="false"
    aria-label="search"
  />
{/if}

<style>
  .ctl {
    min-width: 0;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    /* The pane header sets `cursor: grab` on itself (the whole bar is the drag handle); a text
       field inside it must not claim to be draggable. Set here because Svelte scoping stops the
       header reaching into this component. */
    cursor: auto;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .seg button {
    padding: 2px 7px;
    background: var(--surface);
    border: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-hover);
    color: var(--text);
  }
  /* The one control here that is a sentence rather than a knob — it wraps to nothing and clips
     rather than pushing the rest of the row off screen. */
  .hides {
    min-width: 0;
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 2px 6px;
    border: 1px solid var(--warn-fg, var(--border));
    border-radius: var(--radius-sm);
    background: var(--warn-bg, var(--surface));
    color: var(--warn-fg, var(--text));
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }
</style>

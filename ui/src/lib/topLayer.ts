// A Svelte action that lifts a popup into the browser's **top layer**, placed against the control
// that opened it, so nothing it sits inside can clip it or paint over it.
//
// **Why.** A note's ＋ window lives inside its pane, and a pane cannot let a child out: `.pane` is a
// size container (`container-type`), which makes it the containing block even for `position: fixed`,
// and it clips with `overflow: hidden`. So wherever a note's window was short — two notes stacked on a
// computer, a small phone above the bottom bar — the ＋ window was cut off by the note it belongs to
// (2026-09-11, `decisions.md#ui`). The top layer (the Popover API) is the platform's one way out of
// every clip and stacking order at once. The element stays where it is in the document, so its
// handlers, its styles and `clickOutside` are untouched; only where it is drawn changes.
//
// It opens below the control when the control is in the top half of the screen and above it otherwise
// — `anchorTo`'s rule for the bottom bar's menus — with its right edge under the control's, nudged in
// from either side. How far it may grow is the stylesheet's call (`data-open` and `--opens-at`),
// because only CSS can read the safe-area insets. Where the API is missing — an older WebView, or
// jsdom — this does nothing, and the popup stays where its own CSS puts it.
const GAP = 4;
const GUTTER = 8;

export function topLayer(node: HTMLElement, anchor: HTMLElement | undefined) {
  if (!anchor || typeof node.showPopover !== 'function') return;
  const place = () => {
    const r = anchor.getBoundingClientRect();
    const down = r.bottom <= window.innerHeight / 2;
    const at = Math.round(down ? r.bottom + GAP : window.innerHeight - r.top + GAP);
    node.dataset.open = down ? 'down' : 'up';
    node.style.setProperty('--opens-at', `${at}px`);
    node.style.top = down ? `${at}px` : 'auto';
    node.style.bottom = down ? 'auto' : `${at}px`;
    // Its width is known only once it is shown, which is why this runs after `showPopover`.
    const w = node.offsetWidth;
    const left = Math.max(GUTTER, Math.min(r.right - w, window.innerWidth - GUTTER - w));
    node.style.left = `${Math.round(left)}px`;
  };
  node.setAttribute('popover', 'manual');
  node.showPopover();
  place();
  window.addEventListener('resize', place);
  return {
    destroy() {
      window.removeEventListener('resize', place);
      // Some engines throw when it is already hidden — it left the document first.
      try {
        node.hidePopover();
      } catch {
        /* nothing to hide */
      }
    },
  };
}

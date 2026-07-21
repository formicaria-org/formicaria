// A Svelte action that fires when a pointer lands outside `node` — the behaviour expected on a
// phone (tap elsewhere) and a laptop (click elsewhere) alike. `pointerdown` in the capture phase
// runs before the target's own handlers and covers touch + mouse in one path; the listener is torn
// down with the element it guards. Shared so every popover dismisses identically.
export function clickOutside(node: HTMLElement, onOutside: () => void) {
  const handler = (e: Event) => {
    if (!node.contains(e.target as Node)) onOutside();
  };
  document.addEventListener('pointerdown', handler, true);
  return { destroy: () => document.removeEventListener('pointerdown', handler, true) };
}

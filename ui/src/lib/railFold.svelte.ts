/// **The rail folds away** (2026-11-24; the owner: *"I see no ability of dropping down the left panel
/// with all the views"*). Folded, the page gets the width and the rail is a slim strip with one
/// button to bring it back; Ctrl+\ does either. A view preference of this browser, like the vaults
/// shown and the sections hidden from the rail, so it is kept here and not in the vault.
const KEY = 'fm-rail-folded';

function read(): boolean {
  try {
    return globalThis.localStorage?.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

let folded = $state(read());

export const railFolded = (): boolean => folded;

export function foldRail(on: boolean = !folded): void {
  folded = on;
  try {
    if (on) globalThis.localStorage?.setItem(KEY, '1');
    else globalThis.localStorage?.removeItem(KEY);
  } catch {
    /* a browser that keeps nothing still folds for this visit */
  }
}

/// Ctrl+\ (⌘\ on a Mac): the key editors use for their side panel; nothing in Firefox takes it.
export const isFoldKey = (e: KeyboardEvent): boolean =>
  (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key === '\\';

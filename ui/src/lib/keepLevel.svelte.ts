/// **The open notes and the backup loop, introduced to each other** (2026-10-17,
/// `decisions.md#sync`, *one person's devices are kept level*).
///
/// Three facts cross between them, and each was missing:
///
/// - **Changes arrived.** A note open before a get went on holding the old text and the old
///   version, so the first keystroke after it was refused as stale and shown as *"someone else
///   changed this note"* — the conflict the owner kept meeting between their own two devices.
///   `sync.svelte.ts` counts arrivals here; an open note with nothing unsaved reloads on the count.
/// - **A note is about to be edited.** The loop backs up first when another device has sent to
///   that vault, so the edit starts on the newest text.
/// - **A note was left.** The loop backs up soon, so the other device has it before it is picked up.
///
/// A module rather than props, for the same reason `sync.svelte.ts` is one: the note panel sits
/// under `Pane` under `App`, and every layer would carry a value it does nothing with. The hooks
/// are no-ops until `App` sets them, so a panel rendered alone (in a test) needs nothing.

/// Bumped each time a get brought changes or conflicts into any vault.
export const arrivals = $state({ count: 0 });

export function changesArrived(): void {
  arrivals.count += 1;
}

type Hook = (vault: string) => void;
let editing: Hook = () => {};
let left: Hook = () => {};

/// `App` says what to do on each; called once, from its setup.
export function setLevelHooks(h: { editing: Hook; left: Hook }): void {
  editing = h.editing;
  left = h.left;
}

export function startingToEdit(vault: string): void {
  editing(vault);
}

export function leftNote(vault: string): void {
  left(vault);
}

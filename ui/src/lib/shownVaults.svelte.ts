// **Which vaults to see, on the computer's pages** (the rail's vault chooser, 2026-11-23).
//
// A view preference, like the phone's vault filter, and kept under the same name
// (`fm-hidden-vaults`, `App.svelte`), so one browser remembers one choice. It hides notes from what
// is shown; it says nothing about who may see them (that is which repository holds the file, and
// `fm_app::scope` for a paired device). **Only names that exist can hide anything**: a removed
// vault's name left behind is inert, as in the older frame.
import { vaultList } from './vaults.svelte';

const KEY = 'fm-hidden-vaults';

function stored(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

let kept = $state<string[]>(typeof localStorage === 'undefined' ? [] : stored());

/** The vaults hidden that exist (the names the server is told to leave out). */
export function hiddenVaults(): string[] {
  const names = (vaultList() ?? []).map((v) => v.name);
  return kept.filter((n) => names.includes(n));
}

/** Whether a note of this vault is shown. A note with no vault is always shown. */
export function shownVault(vault: string | null | undefined): boolean {
  return !vault || !hiddenVaults().includes(vault);
}

/** Show or hide a vault. The last one shown cannot be hidden: that would show nothing at all. */
export function toggleVault(name: string): void {
  const names = (vaultList() ?? []).map((v) => v.name);
  const hidden = hiddenVaults();
  if (!hidden.includes(name) && names.filter((n) => !hidden.includes(n)).length <= 1) return;
  kept = kept.includes(name) ? kept.filter((v) => v !== name) : [...kept, name];
  try {
    localStorage.setItem(KEY, JSON.stringify(kept));
  } catch {
    // A browser that will not remember the choice still honours it on this page.
  }
}

// Another tab's choice, followed here: one browser, one choice.
if (typeof window !== 'undefined')
  addEventListener('storage', (e) => {
    if (e.key === KEY) kept = stored();
  });

/** For tests: forget the choice. */
export function resetShownVaults(): void {
  kept = [];
}

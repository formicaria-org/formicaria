// **What a vault is called, as distinct from what it is.**
//
// A vault's *name* is its identity: the write routing key (`MultiStore::route`), the argument every
// vault-taking command carries, and the key behind the persisted view preferences (`hiddenVaults`,
// `fm-board-order`, `fm-card-order`). It is local to a machine, and two devices that cloned the same
// repository can therefore call one audience two different things — the owner's laptop said `vault`
// where the phone said `notes`, so the same audience looked like two vaults depending on the screen.
//
// The *label* is what to show: the repository behind the vault's remote, which both devices agree
// about. `list_vaults` derives it (see `fm_app::dispatch::remote_label`), falling back to the local
// name when there is no remote — or when two vaults would derive the same label, since a duplicate
// label makes the vault filter ambiguous and hiding notes from the wrong audience is worse than
// showing a folder name.
//
// This registry exists so the twenty-odd places that already pass a vault *name* keep passing it:
// they resolve the label here rather than every call site learning about labels. Same identity/display
// split as `authorKey`/label for contributors — see `decisions.md#ui`.

import type { VaultInfo } from './types';

const state = $state<{ labels: Record<string, string> }>({ labels: {} });

/** Feed it the answer from `list_vaults`. Called wherever that lands, and nowhere else. */
export function setVaultLabels(vaults: VaultInfo[]): void {
  const next: Record<string, string> = {};
  for (const v of vaults) {
    if (v.label) next[v.name] = v.label;
  }
  state.labels = next;
}

/** What to show for this vault. The name itself when there is nothing better — never empty. */
export function labelFor(vault: string | null | undefined): string {
  if (!vault) return '';
  return state.labels[vault] ?? vault;
}

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

import { listVaults } from './ipc';
import type { VaultInfo } from './types';

/// **The vault list itself, not only its labels.**
///
/// This module was a label registry with *one* feeder and several bypasses: `App` fed it after
/// `list_vaults`, and then reassigned its own `vaults` after a create without telling it — so the
/// labels described the world before, and a vault added or removed kept its old name on screen
/// until the app was relaunched. The registry was right and the thing it was derived *from* was
/// copied.
///
/// So the list lives here and the labels are derived from it. There is one setter; a component that
/// has a fresh list calls it, and every reader — the switcher, the vault filter, the settings list,
/// `labelFor` — sees the same answer at the same moment, because there is only one.
///
/// `null` is **"not loaded yet"** and is not the same as `[]`, which is "this machine has no
/// vaults" and is the first-run signal. The boot gate turns on that difference (`App.boot.test.ts`:
/// a failed `list_vaults` must never be mistaken for an empty one), so it is kept in the type.
const state = $state<{ list: VaultInfo[] | null }>({ list: null });

/** Feed it the answer from `list_vaults`. **The only writer** — that is the whole point. */
export function setVaults(vaults: VaultInfo[] | null): void {
  state.list = vaults;
}

/** Every vault this caller can see, or `null` while that is still unknown. Reactive. */
export function vaultList(): VaultInfo[] | null {
  return state.list;
}

/// What to show for this vault. The name itself when there is nothing better — never empty.
///
/// Derived from the list rather than from a second map, so a stale label is not a thing that can
/// exist: there is nowhere for one to be kept.
export function labelFor(vault: string | null | undefined): string {
  if (!vault) return '';
  return state.list?.find((v) => v.name === vault)?.label ?? vault;
}

/// Re-read the vault list. **The fetch lives here so a panel does not have to own one.**
///
/// Deliberately *not* "load it once if nobody has". A load-once left a panel showing whatever the
/// list was when something else last fetched it — and the vault list changes underneath a panel
/// every time one is created, removed or renamed. Opening a screen that lists vaults should show
/// the vaults. `list_vaults` is the cheap arm (local `git config` reads, no network), which is why
/// it can be asked on open at all; `backup_status` could not be.
///
/// A failure leaves whatever was there rather than blanking it: a refresh that could not happen is
/// not evidence the vaults went away, and `null` means "still unknown", which is the distinction the
/// boot gate turns on.
export async function refreshVaults(): Promise<void> {
  await listVaults()
    .then(setVaults)
    .catch(() => {});
}

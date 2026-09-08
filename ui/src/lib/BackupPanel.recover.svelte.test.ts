// **A vault removed by mistake is offered back, by name, without anyone remembering it.**
//
// Removing a vault only unregisters it; `forget_vault.rs` has always pinned that the files survive.
// But coming back meant creating a vault whose name landed on the same folder — and on a phone the
// name *is* the address (`resolve_path` turns an empty path into `<root>/<name>`), while every
// screen shows a vault's *label* instead of its name. So the one identity a person needed to
// recover was the one the UI had stopped showing them.
//
// Reported 2026-09-08 by the owner, who removed the wrong of two vaults on a phone and had nothing
// to click. `forget_vault.rs`'s own header carried the assumption this repairs: "a mistaken click
// costs a retyped path" — true on a desktop, and there is no path to retype on a phone.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as mock from './mock';
import BackupPanel from './BackupPanel.svelte';

beforeEach(() => {
  mock.reset();
  vi.clearAllMocks();
});

const panel = () => render(BackupPanel, { onclose: () => {} } as never);

describe('BackupPanel — vaults on this device that are not in the list', () => {
  it('offers the folder back, and says how much is in it', async () => {
    mock.setRecoverable([{ name: 'notes', path: '/data/vaults/notes', notes: 214 }]);
    panel();

    expect(await screen.findByText('On this device, not in your list')).toBeTruthy();
    expect(await screen.findByText('notes')).toBeTruthy();
    // The field that answers the real question: which of these has my work in it.
    expect(await screen.findByText('214 notes')).toBeTruthy();

    await fireEvent.click(await screen.findByRole('button', { name: /add it back/i }));
    await waitFor(() =>
      expect(screen.getByText(/Added “notes” back with its 214 notes/)).toBeTruthy(),
    );
  });

  it('says nothing at all when there is nothing to recover', async () => {
    // The default, and the desktop's permanent state — there is no managed root to scan. A section
    // that is always on screen is a section nobody reads.
    panel();
    // Anchored on something that must render, so this cannot pass before the panel has loaded.
    await screen.findByRole('heading', { name: /back ?up/i });
    expect(screen.queryByText('On this device, not in your list')).toBeNull();
  });
});

// **An iOS build stops opening about seven days after it is installed**, because it is signed with
// the user's own Apple ID and there is no App Store route (`decisions.md#track-m`, *iOS ships as an
// unsigned IPA that each user signs with their own Apple ID*). Someone who is not told that will
// experience it as the app breaking, and the notes going with it.
//
// The notice is permanent rather than a first-run dialog, and these tests pin that: the thing it
// describes recurs weekly for as long as the app is installed, so a notice dismissed once would be
// gone before the first time it mattered.
//
// It is gated on the **OS**, not on a `sideloaded` flag. The OS is a fact; the route is a policy,
// and `platform` will still be true the day the policy changes.

import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import * as mock from './mock';
import SettingsPanel from './SettingsPanel.svelte';

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
  } as never);
}

afterEach(() => mock.reset());

describe('SettingsPanel — the sideload notice', () => {
  it('tells an iOS build that it expires, and that the notes survive it', async () => {
    mock.setPlatform('ios');
    panel();

    expect((await screen.findAllByText(/about 7 days after you installed it/)).length)
      .toBeGreaterThan(0);
    // The reassurance is the half that stops a lapsed signature reading as data loss, so it is
    // asserted rather than left to the wording of the moment.
    expect((await screen.findAllByText(/Your notes are not affected/)).length).toBeGreaterThan(0);
  });

  it('says nothing about expiry on a desktop, where nothing expires', async () => {
    panel();
    // `findAllByText` waits; `queryAllByText` must be given the same chance to be wrong, so the
    // config is awaited via a fact the desktop *does* render before asserting the absence.
    await screen.findAllByText(/vault list/i);
    expect(screen.queryAllByText(/about 7 days after you installed it/)).toHaveLength(0);
  });

  it('warns about media on any phone, not only iOS — a push carries notes, not blobs', async () => {
    mock.setPlatform('android');
    panel();
    // Gated on the managed root rather than the OS: app-private storage goes when the app does,
    // and `blobs/` is gitignored, which is true of both phones.
    expect((await screen.findAllByText(/live only on this phone/)).length).toBeGreaterThan(0);
  });
});

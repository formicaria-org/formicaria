// A watch's activity files in Settings: check a folder, see what would be added, import it once.
// Every verdict is the server's; the mock stands in for `crates/fm-app/src/watch.rs`.
import { afterEach, describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import WatchImport from './WatchImport.svelte';
import { reset } from './mock';

afterEach(() => reset());

async function typeFolder(path: string) {
  const input = (await screen.findByPlaceholderText('~/watch/Activity')) as HTMLInputElement;
  await fireEvent.input(input, { target: { value: path } });
}

describe('watch files in Settings', () => {
  it('imports only after a check, and says what it added', async () => {
    render(WatchImport);
    const importButton = screen.getByRole('button', { name: 'Import' }) as HTMLButtonElement;
    expect(importButton.disabled).toBe(true);
    await typeFolder('~/watch/broken');
    await fireEvent.click(screen.getByRole('button', { name: 'Check' }));
    expect(await screen.findByText(/Would add 3 activities and 6 sets/)).toBeTruthy();
    // A damaged file is named, with what to do.
    expect(screen.getByText(/broken\.fit: .*export the activity again/)).toBeTruthy();
    expect(importButton.disabled).toBe(false);
    await fireEvent.click(importButton);
    expect(await screen.findByText(/Added 3 activities and 6 sets/)).toBeTruthy();
  });

  it('says so when a folder holds nothing to import', async () => {
    render(WatchImport);
    await typeFolder('~/somewhere/else');
    await fireEvent.click(screen.getByRole('button', { name: 'Check' }));
    expect(await screen.findByText(/no \.fit files/)).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Import' }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });
});

// **On the phone** (`decisions.md` 2026-11-20, *The phone takes watch files*): no path to type; one
// folder granted through Android's picker, and a grant taken back is said plainly. The shell is a
// fake shaped as `MainActivity.kt`'s `__fmWatch`.
describe('watch files in Settings, on the phone', () => {
  const state = {
    folder: '',
    readable: true,
    looking: false,
    held: 0,
    waiting: 0,
    tooLarge: 0,
    app: '',
    asked: 0,
  };
  let chosen = 0;
  let calls: string[] = [];
  afterEach(() => {
    delete (globalThis as { __fmWatch?: unknown }).__fmWatch;
  });
  const fake = (s: Partial<typeof state>) => {
    chosen = 0;
    calls = [];
    let now = { ...state, ...s };
    (globalThis as { __fmWatch?: unknown }).__fmWatch = {
      state: () => JSON.stringify(now),
      choose: () => void chosen++,
      forget: () => {},
      look: () => {},
      pending: () => '[]',
      read: () => '',
      done: () => {},
      openApp: () => void calls.push('open'),
      getApp: () => void calls.push('get'),
      sync: () => {
        calls.push('sync');
        now = { ...now, asked: Date.now() };
      },
    };
  };

  it('offers the folder picker, and no path to type', async () => {
    fake({});
    render(WatchImport, { phone: true });
    expect(screen.queryByPlaceholderText('~/watch/Activity')).toBeNull();
    await fireEvent.click(
      await screen.findByRole('button', { name: 'Choose the folder your watch app exports to' }),
    );
    expect(chosen).toBe(1);
  });

  it('says when the folder is no longer readable, and what waits', async () => {
    fake({ folder: 'Gadgetbridge', readable: false, waiting: 1 });
    render(WatchImport, { phone: true });
    expect(await screen.findByText(/no longer readable/)).toBeTruthy();
    expect(screen.getByText(/1 file is still being written/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Stop reading it' })).toBeTruthy();
  });

  // **The watch's app, set up from here** (`decisions.md` 2026-11-20 (night)): fm says whether
  // Gadgetbridge is installed, opens it or its F-Droid page, and asks it to sync. The switches it
  // names are Gadgetbridge's own; the shell (`MainActivity.kt`) sends the messages.
  it('offers Gadgetbridge when it is not installed, with the steps to set it up', async () => {
    fake({});
    render(WatchImport, { phone: true });
    await fireEvent.click(await screen.findByRole('button', { name: 'Get Gadgetbridge' }));
    expect(calls).toEqual(['get']);
    expect(screen.queryByRole('button', { name: 'Sync now' })).toBeNull();
    // With no folder yet, the steps are open, naming Gadgetbridge's own switches.
    expect(screen.getByText(/Auto export FIT tracks/)).toBeTruthy();
    expect(screen.getByText('Allow activity sync trigger')).toBeTruthy();
  });

  it('asks Gadgetbridge to sync, and says when the folder is looked at', async () => {
    fake({ folder: 'Gadgetbridge', app: 'nodomain.freeyourgadget.gadgetbridge' });
    render(WatchImport, { phone: true });
    expect(await screen.findByText('Gadgetbridge is on this phone.')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Open Gadgetbridge' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Sync now' }));
    expect(calls).toEqual(['open', 'sync']);
    expect(await screen.findByText(/Asked Gadgetbridge to sync at/)).toBeTruthy();
    expect(screen.getByText(/looked at\s+again over the next three minutes/)).toBeTruthy();
  });
});

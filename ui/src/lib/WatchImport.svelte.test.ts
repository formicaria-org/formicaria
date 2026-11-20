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
  const state = { folder: '', readable: true, looking: false, held: 0, waiting: 0, tooLarge: 0 };
  let chosen = 0;
  afterEach(() => {
    delete (globalThis as { __fmWatch?: unknown }).__fmWatch;
  });
  const fake = (s: Partial<typeof state>) => {
    chosen = 0;
    (globalThis as { __fmWatch?: unknown }).__fmWatch = {
      state: () => JSON.stringify({ ...state, ...s }),
      choose: () => void chosen++,
      forget: () => {},
      look: () => {},
      pending: () => '[]',
      read: () => '',
      done: () => {},
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
});

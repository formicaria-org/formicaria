// Food and training in Settings: the switch is this device's own, and the food list that comes
// with the app says what it is and what it takes.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import TrackersSection from './TrackersSection.svelte';
import { reset, faults } from './mock';
import * as trackers from './trackers.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  trackers.reload();
});
afterEach(() => {
  vi.unstubAllGlobals();
  reset();
});

describe('food and training in Settings', () => {
  it('is on until this device says otherwise, and the switch is remembered here', async () => {
    render(TrackersSection);
    const sw = (await screen.findByRole('switch')) as HTMLInputElement;
    expect(sw.checked).toBe(true);
    await fireEvent.click(sw);
    expect(trackers.trackersShown()).toBe(false);
    expect(store.get('fm-trackers')).toBe('0');
  });

  it('says what the food list is and what it takes, and offers nothing to get or remove', async () => {
    render(TrackersSection);
    expect(await screen.findByText(/comes with the app/)).toBeTruthy();
    expect(screen.getByText(/8\.\dMB on this device/)).toBeTruthy();
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('says so, and what still works, when the list could not be put in place', async () => {
    faults([{ cmd: 'pack_status', mode: 'reject' }]);
    render(TrackersSection);
    expect(await screen.findByText(/foods can only be added by name/)).toBeTruthy();
  });
});

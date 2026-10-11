// Food and training in Settings: the switch is this device's own, and the food list is never
// fetched or removed without saying what it costs or hands back.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import TrackersSection from './TrackersSection.svelte';
import { reset } from './mock';
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
  it('is off until this device asks, and the switch is remembered here', async () => {
    render(TrackersSection);
    const sw = (await screen.findByRole('switch')) as HTMLInputElement;
    expect(sw.checked).toBe(false);
    await fireEvent.click(sw);
    expect(trackers.trackersShown()).toBe(true);
    expect(store.get('fm-trackers')).toBe('1');
  });

  it('removing the food list takes two presses and says what comes back', async () => {
    render(TrackersSection);
    await fireEvent.click(await screen.findByRole('button', { name: 'Remove the food list' }));
    // Armed, not removed: the list is still described as on this device.
    expect(screen.getByText(/On this device/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Remove it and free 8.1MB' }));
    // Gone — and what it will cost to get again is said before the control that gets it.
    expect(await screen.findByText(/8\.1MB to keep on this device/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Get the food list' })).toBeTruthy();
  });

  it('getting it again puts it back', async () => {
    render(TrackersSection);
    await fireEvent.click(await screen.findByRole('button', { name: 'Remove the food list' }));
    await fireEvent.click(screen.getByRole('button', { name: /^Remove it and free/ }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Get the food list' }));
    await waitFor(() => expect(screen.getByText(/On this device/)).toBeTruthy());
  });
});

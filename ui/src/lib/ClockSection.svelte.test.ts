// Settings → Clock (2026-10-24): Automatic by default; a zone typed and saved becomes the app's.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, expect, it } from 'vitest';
import ClockSection from './ClockSection.svelte';
import * as mock from './mock';
import { appZone, setAppZone } from './appTime';

afterEach(() => {
  mock.reset();
  setAppZone(null);
});

it('is Automatic until a zone is chosen, and a chosen one becomes the app clock', async () => {
  render(ClockSection);
  expect(await screen.findByText(/Automatic: this device's own zone/)).toBeTruthy();
  const box = screen.getByLabelText('time zone') as HTMLInputElement;
  await fireEvent.input(box, { target: { value: 'Asia/Kathmandu' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(await screen.findByText(/Chosen here: times are shown on Asia\/Kathmandu/)).toBeTruthy();
  expect(appZone()).toBe('Asia/Kathmandu');
  await fireEvent.click(screen.getByRole('button', { name: 'Automatic' }));
  expect(await screen.findByText(/Automatic: this device's own zone/)).toBeTruthy();
});

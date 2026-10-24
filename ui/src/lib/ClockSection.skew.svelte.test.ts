// The internet check (2026-10-24): a warning only, from an answer the app already had.
import { render, screen } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';

vi.mock('./ipc', async (orig) => ({
  ...(await orig<typeof import('./ipc')>()),
  clock: vi.fn(async () => ({
    zone: 'Asia/Kathmandu',
    source: 'automatic',
    system: 'Asia/Kathmandu',
    skew_seconds: 600,
    skew_from: 'oauth2.googleapis.com',
  })),
}));

const { default: ClockSection } = await import('./ClockSection.svelte');

it("says when this device's clock is off the internet's, and by how much", async () => {
  render(ClockSection);
  expect(
    await screen.findByText(
      /This device's clock is 10 minutes\s+ahead of the internet's \(oauth2\.googleapis\.com\)/,
    ),
  ).toBeTruthy();
});

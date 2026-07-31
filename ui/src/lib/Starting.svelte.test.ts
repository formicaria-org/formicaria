// The startup screen exists for one failure mode: a backend that never answers, which used to
// render as *nothing at all* — a blank window on a phone with no console, no stdout and no way to
// tell "still opening" from "broken" (the owner's, 2026-07-31). So what is pinned here is exactly
// that contract: silent while a normal launch completes, and never silent afterwards.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import Starting from './Starting.svelte';

describe('Starting', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('shows nothing at first, so a fast launch never flashes it', () => {
    render(Starting, { onretry: () => {} });
    expect(screen.queryByText(/Opening your vaults/)).toBeNull();
  });

  it('speaks once the wait stops being plausible', async () => {
    render(Starting, { onretry: () => {} });
    await vi.advanceTimersByTimeAsync(800);
    expect(screen.getByText(/Opening your vaults/)).toBeTruthy();
  });

  it("renders the backend's own reason and offers a retry", async () => {
    const onretry = vi.fn();
    render(Starting, { error: 'could not open vaults: no such file', attempts: 3, onretry });
    await vi.advanceTimersByTimeAsync(800);
    // Verbatim, because on a phone this is the only place the sentence can be read at all.
    expect(screen.getByText(/no such file/)).toBeTruthy();
    expect(screen.getByText(/Tried 3 times/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(onretry).toHaveBeenCalled();
  });
});

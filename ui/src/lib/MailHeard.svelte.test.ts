// A reply that only confirms a meeting proposes nothing — and must still say it was read
// (2026-10-16: a collaborator's "sounds good" was read within a minute, and looked like it never arrived).

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { MailStatus } from './ipc';

let status: MailStatus | Error;
vi.mock('./ipc', async (orig) => ({
  ...(await orig<typeof import('./ipc')>()),
  mailStatus: vi.fn(async () => {
    if (status instanceof Error) throw status;
    return status;
  }),
}));

const { default: MailHeard } = await import('./MailHeard.svelte');

const base: MailStatus = {
  configured: true,
  connected: true,
  email: 'me@example.org',
  label: 'formicaria',
  since: '2026-10-01',
  vault: 'v',
  read: 12,
  lastChecked: Math.floor(Date.now() / 1000) - 30,
  reading: false,
  last: { at: 0, messages: 0 },
};

describe('what the assistant made of the mail', () => {
  it('shows a verdict that changed nothing, and opens its conversation', async () => {
    status = {
      ...base,
      heard: [
        {
          note: 'C1',
          title: 'Sara Lodi — Collaborator',
          latest: '2026-10-16T09:13',
          said: 'no change — the meeting is still on Fri 23 Oct at 16:00',
          at: Math.floor(Date.now() / 1000),
        },
      ],
    };
    const onopen = vi.fn();
    render(MailHeard, { onopen });
    expect(await screen.findByText(/checked just now/)).toBeTruthy();
    expect(
      await screen.findByText('no change — the meeting is still on Fri 23 Oct at 16:00'),
    ).toBeTruthy();
    // The email's own date, not when it was read; and, it being days old, that nothing new came
    // (2026-10-20: "the email is read just now but that was 4 days ago?").
    expect(screen.getAllByText(/16.*09:13/).length).toBeGreaterThan(0);
    expect(screen.getByText(/No new email in these conversations since/)).toBeTruthy();
    await fireEvent.click(screen.getByText('Sara Lodi — Collaborator'));
    expect(onopen).toHaveBeenCalledWith('C1');
  });

  /// 2026-10-16: Google ended the sign-in (7 days, a project in testing) and mail stopped; the only
  /// word of it was in Settings, and this section hid itself while disconnected.
  it('says loudly that mail stopped, and why, with the way to sign in again', async () => {
    status = {
      ...base,
      connected: false,
      last: { at: 0, error: 'Google ended the sign-in (it does this every 7 days).' },
    };
    render(MailHeard, { onopen: () => {} });
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toMatch(/stopped/);
    expect(alert.textContent).toMatch(/Google ended the sign-in/);
    expect(screen.getByRole('button', { name: 'Sign in with Google' })).toBeTruthy();
    // And the way to stop it recurring: the Google Cloud page, and the manual's steps.
    expect(screen.getByRole('link', { name: 'publish the app' }).getAttribute('href')).toBe(
      'https://console.cloud.google.com/auth/audience',
    );
    expect(screen.getByRole('link', { name: 'how, step by step' }).getAttribute('href')).toBe(
      '/manual/user/mail.html#stop-the-weekly-sign-out',
    );
  });

  it('shows nothing where there is no mail', async () => {
    status = new Error('no such command');
    const { container } = render(MailHeard, { onopen: () => {} });
    await Promise.resolve();
    expect(container.querySelector('.mail-heard')).toBeNull();
  });
});

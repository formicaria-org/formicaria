import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import MailSection from './MailSection.svelte';

describe('MailSection', () => {
  it('walks set-up → sign-in → reading, and never shows the secret back', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    render(MailSection);
    // Step 1: the client. Save stays off until both values are there.
    const save = await screen.findByRole('button', { name: 'Save' });
    expect((save as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.input(screen.getByPlaceholderText('…apps.googleusercontent.com'), {
      target: { value: '123-abc.apps.googleusercontent.com' },
    });
    await fireEvent.input(screen.getByPlaceholderText('GOCSPX-…'), {
      target: { value: 'GOCSPX-very-secret' },
    });
    await fireEvent.click(save);

    // Step 2: sign in. The tab is opened before the await, so a popup blocker allows it.
    const signIn = await screen.findByRole('button', { name: 'Sign in with Google' });
    await fireEvent.click(signIn);
    expect(open).toHaveBeenCalled();
    await waitFor(() => expect(screen.getByText('Connected')).toBeTruthy(), { timeout: 5000 });
    expect(document.body.textContent).not.toContain('GOCSPX-very-secret');

    // Step 3: reading says what it did, in plain counts.
    await fireEvent.click(screen.getByRole('button', { name: 'Read now' }));
    await waitFor(() =>
      expect(screen.getByText(/2 new conversations, 1 with replies/)).toBeTruthy(),
    );
    expect(screen.getByText(/Invites: 1 new, 0 moved, 0 cancelled/)).toBeTruthy();

    // Disconnecting is armed first, and says what it keeps.
    await fireEvent.click(screen.getByRole('button', { name: 'Disconnect…' }));
    expect(screen.getByText(/The notes already made stay/)).toBeTruthy();
    open.mockRestore();
  });

  it('refuses a client ID that is not one, as a sentence', async () => {
    render(MailSection);
    // The mock keeps the first test's saved client, so the field is found by its label.
    await screen.findByRole('button', { name: 'Save' });
    await fireEvent.input(screen.getByLabelText('Client ID'), {
      target: { value: 'not-a-client-id' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect((await screen.findByRole('alert')).textContent).toContain('apps.googleusercontent.com');
  });
});

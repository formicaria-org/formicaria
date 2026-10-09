import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CalendarsSection from './CalendarsSection.svelte';

describe('CalendarsSection', () => {
  it('adds a calendar, reads it at once, and never shows the private address back', async () => {
    render(CalendarsSection);
    await screen.findByText('No calendars yet.');

    await fireEvent.input(screen.getByPlaceholderText('work'), { target: { value: 'work' } });
    const secret = 'https://calendar.google.com/calendar/ical/me/private-0123abcd/basic.ics';
    await fireEvent.input(screen.getByPlaceholderText('https://calendar.google.com/…/basic.ics'), {
      target: { value: secret },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Add calendar' }));

    // Read straight away, and the ones left out are said rather than hidden.
    await waitFor(() => expect(screen.getByText(/2 new, 0 moved, 0 cancelled/)).toBeTruthy());
    expect(screen.getByText(/5 already over/)).toBeTruthy();
    expect(screen.getByText(/1 repeating with a pattern formicaria cannot read yet/)).toBeTruthy();
    expect(screen.getByText(/2 repeating meetings are shown for the next two months/)).toBeTruthy();
    // The host identifies it; the secret part never appears.
    expect(document.body.textContent).toContain('calendar.google.com');
    expect(document.body.textContent).not.toContain('private-0123abcd');
    // The paste field was cleared, so the secret does not linger on screen either.
    expect(
      (screen.getByPlaceholderText('https://calendar.google.com/…/basic.ics') as HTMLInputElement)
        .value,
    ).toBe('');
  });

  it('says why an address was refused, as a sentence', async () => {
    render(CalendarsSection);
    // The mock keeps the first test's calendar; what matters here is the form, not the list.
    await screen.findByRole('button', { name: 'Add calendar' });
    await fireEvent.input(screen.getByPlaceholderText('work'), { target: { value: 'x' } });
    await fireEvent.input(screen.getByPlaceholderText('https://calendar.google.com/…/basic.ics'), {
      target: { value: 'calendar.google.com/x' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Add calendar' }));
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('starts with https://');
  });
});

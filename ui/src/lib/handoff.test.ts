import { describe, expect, it } from 'vitest';
import { calendarEvent, eventDates, gmailCompose, icsEvent } from './handoff';

describe('handing a note to a service outside', () => {
  it('opens Gmail compose with the subject and the text', () => {
    const u = new URL(gmailCompose('Lab visit', 'Tuesday works.\nAt ten.'));
    expect(u.host).toBe('mail.google.com');
    expect(u.searchParams.get('view')).toBe('cm');
    expect(u.searchParams.get('su')).toBe('Lab visit');
    expect(u.searchParams.get('body')).toBe('Tuesday works.\nAt ten.');
  });

  it('turns a note date into a calendar span', () => {
    expect(eventDates('2026-11-20')).toBe('20261120/20261121');
    expect(eventDates('2026-11-20', '2026-11-22')).toBe('20261120/20261123');
    expect(eventDates('2026-11-20T14:30')).toBe('20261120T143000/20261120T153000');
    expect(eventDates('2026-11-20T14:30', '2026-11-20T16:00')).toBe(
      '20261120T143000/20261120T160000',
    );
    expect(eventDates('2026-11-20T23:30')).toBe('20261120T233000/20261120T235900');
    expect(eventDates('soon')).toBeNull();
  });

  it('opens a prefilled calendar event, or makes an .ics for another calendar', () => {
    const u = new URL(
      calendarEvent({
        title: 'Lab visit',
        start: '2026-11-20T10:00',
        details: 'Bring the samples.',
      })!,
    );
    expect(u.host).toBe('calendar.google.com');
    expect(u.searchParams.get('action')).toBe('TEMPLATE');
    expect(u.searchParams.get('dates')).toBe('20261120T100000/20261120T110000');
    expect(calendarEvent({ title: 'x', start: 'never' })).toBeNull();
    const ics = icsEvent({ title: 'Lab, visit', start: '2026-11-20', uid: 'n1@formicaria' })!;
    expect(ics).toContain('DTSTART;VALUE=DATE:20261120');
    expect(ics).toContain('SUMMARY:Lab\\, visit');
  });
});

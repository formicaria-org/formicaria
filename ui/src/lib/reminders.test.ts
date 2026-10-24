import { expect, test } from 'vitest';
import { msOf, textOf } from './reminders.svelte';

test('a reminder says whether it starts or is due, and when', () => {
  expect(textOf({ what: 'start', all_day: false })).toBe('Starts now');
  expect(textOf({ what: 'due', all_day: true })).toBe('Due today');
  expect(textOf({ what: 'due', when: '10:00', all_day: false })).toBe('Due at 10:00');
  expect(textOf({ what: 'start', when: 'that day', all_day: true })).toBe('Starts that day');
});

test("a reminder's moment is the server's, worked out on the app clock", () => {
  // 15:30 on a +05:45 clock is 09:45 UTC; the page never re-reads it in its own zone (2026-10-24).
  const atMs = Date.UTC(2026, 9, 14, 9, 45);
  expect(msOf({ at: '2026-10-14T15:30', at_ms: atMs })).toBe(atMs);
});

test('from a server older than 0.19, with no moment, the wall time is read as before', () => {
  const at = new Date(msOf({ at: '2026-10-14T15:30' }));
  expect([at.getFullYear(), at.getMonth(), at.getDate(), at.getHours(), at.getMinutes()]).toEqual([
    2026, 9, 14, 15, 30,
  ]);
});

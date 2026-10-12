import { expect, test } from 'vitest';
import { msOf, textOf } from './reminders.svelte';

test('a reminder says whether it starts or is due, and when', () => {
  expect(textOf({ what: 'start', all_day: false })).toBe('Starts now');
  expect(textOf({ what: 'due', all_day: true })).toBe('Due today');
  expect(textOf({ what: 'due', when: '10:00', all_day: false })).toBe('Due at 10:00');
  expect(textOf({ what: 'start', when: 'that day', all_day: true })).toBe('Starts that day');
});

test('a moment written in the notes is this device’s own time', () => {
  const at = new Date(msOf('2026-10-14T15:30'));
  expect([at.getFullYear(), at.getMonth(), at.getDate(), at.getHours(), at.getMinutes()]).toEqual([
    2026, 9, 14, 15, 30,
  ]);
});

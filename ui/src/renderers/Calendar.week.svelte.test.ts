// The week is a time grid: a note with a clock time sits in its day's column at that time, and a
// note with only a date stays in the strip above the hours. The owner, 2026-10-11: *in the weekly
// view I should see the tasks in their time slot, not just a list*.
import { render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Calendar from './Calendar.svelte';
import { addDays, weekOf } from '../lib/calendar';
import { appToday } from '../lib/appTime';
import type { ObjectMeta } from '../lib/types';

const week = weekOf(appToday());
const tue = week[1].date;
const wed = week[2].date;
const card = (id: string, start: string | null, due: string): ObjectMeta =>
  ({ id, title: id, preview: null, vault: 'v', start, due, hard: false }) as unknown as ObjectMeta;

function show(cards: ObjectMeta[], range: 'week' | 'month' = 'week') {
  const onopen = vi.fn();
  const { container } = render(Calendar, { cards, onopen, range });
  return { container, onopen };
}

describe('Calendar — the week as a time grid', () => {
  it('puts a timed note in its own day, at its time and as tall as it is long', () => {
    const { container, onopen } = show([card('meeting', `${tue}T17:30`, `${tue}T18:30`)]);
    const col = container.querySelector(`.day-col[data-day="${tue}"]`)!;
    const block = col.querySelector<HTMLElement>('.block')!;
    expect(block.textContent).toContain('17:30–18:30');
    expect(block.textContent).toContain('meeting');
    expect(block.getAttribute('style')).toContain(`top: calc(${17 * 60 + 30} / 60 * var(--hour))`);
    expect(block.getAttribute('style')).toContain('height: calc(60 / 60 * var(--hour))');
    // Drawn once: not also as a bar above the hours.
    expect(container.querySelectorAll('.event').length).toBe(0);
    block.click();
    expect(onopen).toHaveBeenCalledWith('meeting');
  });

  it('keeps a note with a date and no time, or one across days, above the hours', () => {
    const { container } = show([card('deadline', null, wed), card('trip', tue, `${wed}T10:00`)]);
    expect(container.querySelectorAll('.block').length).toBe(0);
    const bars = [...container.querySelectorAll('.week-head .event')].map((e) => e.textContent);
    expect(bars.some((t) => t?.includes('deadline'))).toBe(true);
    expect(bars.some((t) => t?.includes('trip'))).toBe(true);
  });

  it('shows every hour of the day and all seven days', () => {
    const { container } = show([]);
    expect(container.querySelectorAll('.hour-row').length).toBe(24);
    expect(container.querySelectorAll('.day-col').length).toBe(7);
    expect(container.querySelectorAll('.day-col.is-today .now-line').length).toBe(1);
  });

  it("moves with the week: next week shows next week's notes in their slots", async () => {
    const next = addDays(tue, 7);
    const { container } = show([card('later', null, `${next}T09:00`)]);
    expect(container.querySelectorAll('.block').length).toBe(0);
    container.querySelector<HTMLElement>('button[aria-label="next"]')!.click();
    await Promise.resolve();
    await new Promise((r) => setTimeout(r, 0));
    expect(container.querySelector(`.day-col[data-day="${next}"] .block`)).not.toBeNull();
  });

  it('leaves the month as it was: bars by day, no hours', () => {
    const { container } = show([card('meeting', `${tue}T17:30`, `${tue}T18:30`)], 'month');
    expect(container.querySelectorAll('.hour-row').length).toBe(0);
    expect(container.querySelectorAll('.event').length).toBe(1);
  });
});

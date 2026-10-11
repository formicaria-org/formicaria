// A fortnight of bars: how many notes fall on each day, and a tap that keeps one day's.
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import DayStrip from './DayStrip.svelte';
import { addDays, dayLabel, daysBetween, today } from './units';

const days = daysBetween(today(), addDays(today(), 13));

describe('the fortnight strip', () => {
  it('counts the notes on each day, and leaves out the ones with no day', () => {
    render(DayStrip, {
      caption: 'Due in the next two weeks',
      days,
      on: [days[0], days[0], days[3], null, '1999-01-01'],
      chosen: null,
      onpick: () => {},
    });
    expect(screen.getByRole('button', { name: `${dayLabel(days[0])}: 2` })).toBeTruthy();
    expect(screen.getByRole('button', { name: `${dayLabel(days[3])}: 1` })).toBeTruthy();
    expect(screen.getByRole('button', { name: `${dayLabel(days[1])}: 0` })).toBeTruthy();
    expect(screen.getAllByRole('button')).toHaveLength(14);
  });

  it('a tap picks a day, and a tap on the day already picked lets them all back', async () => {
    const onpick = vi.fn();
    const { rerender } = render(DayStrip, {
      caption: 'Due',
      days,
      on: [days[2]],
      chosen: null,
      onpick,
    });
    await fireEvent.click(screen.getByRole('button', { name: `${dayLabel(days[2])}: 1` }));
    expect(onpick).toHaveBeenLastCalledWith(days[2]);

    await rerender({ chosen: days[2] });
    const picked = screen.getByRole('button', { name: `${dayLabel(days[2])}: 1` });
    expect(picked.getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(picked);
    expect(onpick).toHaveBeenLastCalledWith(null);
  });

  it('draws nothing when no note falls in the fortnight', () => {
    render(DayStrip, { caption: 'Due', days, on: [null], chosen: null, onpick: () => {} });
    expect(screen.queryByRole('region')).toBeNull();
    expect(screen.queryAllByRole('button')).toHaveLength(0);
  });
});

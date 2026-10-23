import { describe, expect, it } from 'vitest';
import { city, zoneLabel } from './zoneLabel';

describe('the agenda says which clock its times are on', () => {
  it('names the city and the offset on that day, and the notes’ own clock in the tooltip', () => {
    // 27 October 2026: Berlin has left summer time, so GMT+1.
    const l = zoneLabel(
      'Europe/Berlin',
      ['+09:00', null, '+09:00'],
      new Date('2026-10-27T09:00:00Z'),
    );
    expect(l.text).toBe('Berlin · GMT+1');
    expect(l.title).toContain('Europe/Berlin');
    expect(l.title).toContain('+09:00');
    expect(l.title.match(/\+09:00/g)).toHaveLength(1);
  });

  it('reads a multi-part zone as a person would say it', () => {
    expect(city('America/Argentina/Buenos_Aires')).toBe('Buenos Aires');
    expect(zoneLabel('UTC', []).title).not.toContain('keep theirs');
  });
});

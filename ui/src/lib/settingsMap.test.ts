import { describe, expect, it } from 'vitest';
import { CATEGORIES, isCat, labelOf, matches, shown } from './settingsMap';

const base = { actions: false, cat: 'look' as const, query: '', text: '' };

describe('how Settings is divided', () => {
  it('has its categories, each with a name', () => {
    expect(CATEGORIES.map((c) => c.id)).toEqual([
      'look',
      'keyboard',
      'clock',
      'vaults',
      'notes',
      'assistant',
      'connections',
      'about',
    ]);
    expect(labelOf('vaults')).toBe('Vaults & backup');
    expect(labelOf('notes')).toBe('Food & reminders');
    expect(isCat('about')).toBe(true);
    expect(isCat('actions')).toBe(false);
  });

  it('shows one category at a time', () => {
    expect(shown({ ...base, place: 'look' })).toBe(true);
    expect(shown({ ...base, place: 'about' })).toBe(false);
  });

  it('never shows the actions in Settings, and only the actions in their own window', () => {
    expect(shown({ ...base, place: 'actions' })).toBe(false);
    expect(shown({ ...base, place: 'actions', query: 'new note', text: 'New note' })).toBe(false);
    expect(shown({ ...base, actions: true, place: 'actions' })).toBe(true);
    expect(shown({ ...base, actions: true, place: 'look' })).toBe(false);
  });

  it("searches every category at once, by any of a section's words", () => {
    const text = 'This machine Backs up by itself: a little after you stop writing';
    expect(shown({ ...base, place: 'notes', query: 'backup itself', text })).toBe(false);
    expect(shown({ ...base, place: 'notes', query: 'backs itself', text })).toBe(true);
    expect(shown({ ...base, place: 'look', query: 'backs itself', text: 'Theme Dark' })).toBe(
      false,
    );
    // A group's label is not a result by itself.
    expect(
      shown({ ...base, place: 'about', query: 'read', text: 'read-only', heading: true }),
    ).toBe(false);
  });

  it('matches whatever the case and whatever the order of the words', () => {
    expect(matches('Look for newer versions', 'VERSIONS newer')).toBe(true);
    expect(matches('Look for newer versions', 'older')).toBe(false);
    expect(matches('anything', '   ')).toBe(true);
  });
});

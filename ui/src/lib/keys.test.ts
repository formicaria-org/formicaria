// The binding layer, and specifically the property that made it necessary: navigation must
// survive a focused editor, while anything that would eat a keystroke must not.
import { describe, expect, it } from 'vitest';
import * as keys from './keys';

const ev = (init: Partial<KeyboardEvent> & { key: string }) =>
  ({ altKey: false, shiftKey: false, ctrlKey: false, metaKey: false, ...init }) as KeyboardEvent;

describe('bindings', () => {
  it('matches a modifier chord exactly, not loosely', () => {
    const b = { key: ']', mod: true };
    expect(keys.matches(ev({ key: ']', ctrlKey: true }), b)).toBe(true);
    expect(keys.matches(ev({ key: ']', metaKey: true }), b)).toBe(true); // Cmd is the same intent
    expect(keys.matches(ev({ key: ']' }), b)).toBe(false); // bare key must not fire
    expect(keys.matches(ev({ key: ']', ctrlKey: true, shiftKey: true }), b)).toBe(false);
  });

  it('treats an empty key as unbound rather than matching everything', () => {
    expect(keys.matches(ev({ key: 'a' }), { key: '' })).toBe(false);
  });

  /** The bug this whole module exists to fix: with a note open, nothing could reach another
   *  view. Navigation is exempt from the typing guard — so it MUST require a modifier, or it
   *  would swallow ordinary typing instead. */
  it('every BOUND command allowed while typing needs a modifier', () => {
    for (const cmd of keys.WHILE_TYPING) {
      const b = keys.DEFAULTS[cmd];
      if (!b.key) continue; // unbound: it cannot eat anything
      expect(b.mod || b.alt, `${cmd} would eat a keystroke while typing`).toBeTruthy();
    }
  });

  /** The browser takes these before the page sees them: Ctrl+digit switches tab, Ctrl+0 resets
   *  zoom, Ctrl +/- zooms. The first defaults used Ctrl+1/2/9/0 — layout-stable, and every one
   *  already spoken for, so pressing them zoomed or changed tab. Layout-safe is necessary and
   *  not sufficient. */
  it('ships no default the browser has already claimed', () => {
    for (const cmd of Object.keys(keys.DEFAULTS) as keys.Command[]) {
      expect(keys.reserved(keys.DEFAULTS[cmd]), `${cmd} is a browser shortcut`).toBe(false);
    }
  });

  it('recognises a reserved binding the user tries to set', () => {
    expect(keys.reserved({ key: '0', mod: true })).toBe(true);
    expect(keys.reserved({ key: '1', mod: true })).toBe(true);
    // A modifier the browser does not use makes it ours again.
    expect(keys.reserved({ key: '1', mod: true, alt: true })).toBe(false);
    expect(keys.reserved({ key: '.', mod: true })).toBe(false);
  });

  it('exempts navigation while typing, and does not exempt note creation', () => {
    expect(keys.WHILE_TYPING.has('nextPane')).toBe(true);
    expect(keys.WHILE_TYPING.has('prevPane')).toBe(true);
    // `c` types a `c`. If this were exempt, writing prose would create notes.
    expect(keys.WHILE_TYPING.has('newNote')).toBe(false);
    expect(keys.WHILE_TYPING.has('focusSearch')).toBe(false);
  });

  it('ships no conflicting defaults', () => {
    const d = keys.DEFAULTS;
    for (const cmd of Object.keys(d) as keys.Command[]) {
      expect(keys.conflict(d, cmd, d[cmd]), `${cmd} clashes by default`).toBeNull();
    }
  });

  it('detects a clash the user creates', () => {
    const map = { ...keys.DEFAULTS };
    const clash = { key: 'k', mod: true }; // already the palette
    expect(keys.conflict(map, 'nextPane', clash)).toBe('palette');
  });

  it('reads a binding out of a keypress', () => {
    const b = keys.fromEvent(ev({ key: 'W', ctrlKey: true, shiftKey: true }));
    expect(b).toEqual({ key: 'w', mod: true, alt: false, shift: true });
  });

  it('describes a binding the way a human reads it', () => {
    expect(keys.describe({ key: 'w', mod: true, shift: true }, 'Linux x86_64')).toBe('Ctrl+Shift+W');
    expect(keys.describe({ key: 'k', mod: true }, 'MacIntel')).toBe('⌘K');
    expect(keys.describe({ key: '' })).toBe('unbound');
  });
});

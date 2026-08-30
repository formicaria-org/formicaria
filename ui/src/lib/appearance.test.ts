/// The parts of theming that decide something, tested without a browser.
///
/// The escape hatch's armed lifecycle is in here too, deliberately: it is the one layer that is
/// pure JS and therefore *fully* testable. What cannot be tested here — that the injected `<style>`
/// survives the real CSP, and that the escape control outlives a hostile `!important` — is stated
/// as such rather than asserted against jsdom, which applies no CSP and does not implement the
/// cascade faithfully. This repo lost PDF rendering for months to exactly that mistake.
import { beforeEach, afterEach, describe, expect, test, vi } from 'vitest';
import {
  SUPPORTED,
  toCss,
  fromCss,
  apply,
  clear,
  arm,
  disarm,
  wasArmed,
  readSelection,
  writeSelection,
} from './appearance';

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  clear();
});
afterEach(() => vi.unstubAllGlobals());

describe('the form and the file are the same thing', () => {
  test('what the form writes, the form can read back', () => {
    const prefs = { accent: '#8a5a2b', textBase: '1rem', fontSans: 'Georgia, serif' };
    const back = fromCss(toCss(prefs));
    expect(back.extra).toBe(false);
    expect(back.prefs).toEqual(prefs);
  });

  test('one accent colour answers the four tokens that must agree', () => {
    const css = toCss({ accent: '#8a5a2b' });
    for (const t of ['--accent', '--accent-hover', '--link', '--focus-ring']) {
      expect(css).toContain(`${t}: #8a5a2b;`);
    }
  });

  test('an empty form writes nothing at all, not an empty rule', () => {
    expect(toCss({})).toBe('');
  });

  test('a hand-written theme is reported as more than the form can say', () => {
    // A selector, a media query, and a token outside the promised surface: each on its own must be
    // enough to stop the form rewriting the file. Flattening someone's work silently is the failure
    // `save_view`'s refuse-don't-flatten rule exists to prevent.
    for (const css of [
      '.pane-head { display: none; }',
      '@media (max-width: 40rem) { :root { --bg: #000; } }',
      ':root { --card-bg: #123456; }',
      ':root { --bg: #111; }\n.read { font-size: 2rem; }',
    ]) {
      expect(fromCss(css).extra, css).toBe(true);
    }
  });

  test('comments and blank files are not "extra"', () => {
    expect(fromCss('/* my theme */').extra).toBe(false);
    expect(fromCss('   ').extra).toBe(false);
  });

  test('the supported surface is the semantic layer, not the legacy aliases', () => {
    for (const t of ['bg', 'surface', 'text', 'accent', 'font-sans', 'space-1', 'u-overdue']) {
      expect(SUPPORTED).toContain(t);
    }
    // Layer (3) indirections and layout internals are outside the promise.
    for (const t of ['card-bg', 'panel', 'tag-bg', 'rail-w', 'header-h', 'bp-narrow', 'safe-top']) {
      expect(SUPPORTED).not.toContain(t);
    }
  });
});

describe('applying', () => {
  test('the theme goes in as text, never as markup', () => {
    apply(':root { --bg: #f4f1ea; }');
    const el = document.getElementById('fm-theme') as HTMLStyleElement;
    expect(el.tagName).toBe('STYLE');
    expect(el.textContent).toBe(':root { --bg: #f4f1ea; }');
    // Applying again replaces rather than stacking, or every keystroke in the editor would leave a
    // new <style> behind and the last one would not necessarily win.
    apply(':root { --bg: #000; }');
    expect(document.querySelectorAll('#fm-theme').length).toBe(1);
    expect(document.getElementById('fm-theme')!.textContent).toBe(':root { --bg: #000; }');
  });

  test('clearing removes it entirely', () => {
    apply(':root {}');
    clear();
    expect(document.getElementById('fm-theme')).toBeNull();
  });
});

describe('the escape hatch', () => {
  test('a run nobody could interact with leaves the flag armed', () => {
    arm();
    expect(wasArmed()).toBe(true); // the app is closed here, having never been touched
  });

  test('the first thing the user does disarms it', () => {
    arm();
    document.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    expect(wasArmed()).toBe(false);
  });

  test('a keystroke counts too — a phone with no pointer still gets out', () => {
    arm();
    document.dispatchEvent(new Event('keydown', { bubbles: true }));
    expect(wasArmed()).toBe(false);
  });

  test('once disarmed it stays disarmed for the rest of the session', () => {
    arm();
    document.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    document.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    expect(wasArmed()).toBe(false);
  });

  test('disarm is idempotent and safe with nothing armed', () => {
    disarm();
    expect(wasArmed()).toBe(false);
  });
});

describe('which theme is on', () => {
  test('a selection round-trips, and clearing it removes it', () => {
    writeSelection({ vault: 'personal', name: 'writing-desk' });
    expect(readSelection()).toEqual({ vault: 'personal', name: 'writing-desk' });
    writeSelection(null);
    expect(readSelection()).toBeNull();
  });

  test('a corrupt selection is no selection, not a crash', () => {
    store.set('fm-appearance', '{not json');
    expect(readSelection()).toBeNull();
    store.set('fm-appearance', '{"vault":"v"}');
    expect(readSelection()).toBeNull();
  });

  test('storage that throws does not stop the app', () => {
    // A private window, or a WebView with site data blocked. A theme is a nicety; failing to read
    // one must never be the reason the app will not start.
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('denied');
      },
      setItem: () => {
        throw new Error('denied');
      },
      removeItem: () => {
        throw new Error('denied');
      },
    });
    expect(() => readSelection()).not.toThrow();
    expect(readSelection()).toBeNull();
    expect(() => writeSelection({ vault: 'v', name: 'n' })).not.toThrow();
    expect(() => arm()).not.toThrow();
    expect(wasArmed()).toBe(false);
  });
});

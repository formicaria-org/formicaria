// Writing the same theme again is not a new stylesheet (2026-10-18): on a phone the whole page's
// style was worked out again after every refresh.
import { describe, expect, it } from 'vitest';
import { apply, clear } from './appearance';

describe('apply', () => {
  it('leaves the sheet alone when the theme is unchanged, and writes it when it is not', () => {
    clear();
    apply(':root { --accent: red; }');
    const el = document.head.querySelector('style:last-of-type')!;
    const node = el.firstChild;
    apply(':root { --accent: red; }');
    expect(el.firstChild).toBe(node);
    apply(':root { --accent: blue; }');
    expect(el.textContent).toBe(':root { --accent: blue; }');
    clear();
    apply(':root { --accent: blue; }');
    expect(document.head.textContent).toContain('--accent: blue');
    clear();
  });
});

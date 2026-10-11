import { describe, expect, it } from 'vitest';
import { isTyping } from './typing';

const el = (html: string) => {
  const host = document.createElement('div');
  host.innerHTML = html; // sink-ok: a fixed string in a test
  document.body.append(host);
  return host.firstElementChild as HTMLElement;
};

describe('whether someone is typing', () => {
  it('is yes in a field, a text box and a list to choose from', () => {
    for (const html of ['<input />', '<textarea></textarea>', '<select></select>'])
      expect(isTyping(el(html))).toBe(true);
  });

  /// The editor that renders as you write is an editable element, not a text box. A bare `c`
  /// typed there opened a new note until this was asked of it too.
  it('is yes in anything the browser lets you edit, and in what is inside it', () => {
    const editor = el(
      '<div contenteditable="true"><div class="cm-line"><span>word</span></div></div>',
    );
    expect(isTyping(editor)).toBe(true);
    expect(isTyping(editor.querySelector('span'))).toBe(true);
  });

  it('is no on a button, on the page, and on nothing', () => {
    expect(isTyping(el('<button>go</button>'))).toBe(false);
    expect(isTyping(document.body)).toBe(false);
    expect(isTyping(null)).toBe(false);
  });
});

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { KEYBOARD_EVENT } from './keyboard';
import { TYPING_ATTR, typesText, watchTyping } from './barAside';

const root = document.documentElement;
const typing = () => root.hasAttribute(TYPING_ATTR);

function keyboard(px: number) {
  root.style.setProperty('--kb', `${px}px`);
  window.dispatchEvent(new Event(KEYBOARD_EVENT));
}

const tick = () => new Promise((r) => setTimeout(r, 5));

describe('typing with the keyboard up', () => {
  let stop: () => void = () => {};
  let bar: HTMLElement;

  beforeEach(() => {
    document.body.innerHTML = `
      <main>
        <textarea id="journal"></textarea>
        <input id="search" type="search" />
        <input id="done" type="checkbox" />
        <div id="editor" contenteditable="true"><p id="line">text</p></div>
      </main>
      <header class="bar"><input id="bar-search" type="search" /></header>`;
    bar = document.querySelector('.bar') as HTMLElement;
    stop = watchTyping((el) => bar.contains(el));
  });
  afterEach(() => {
    stop();
    keyboard(0);
    document.body.innerHTML = '';
  });

  it('is set while a text field has the focus and the keyboard is up', () => {
    (document.getElementById('journal') as HTMLElement).focus();
    expect(typing()).toBe(false); // no keyboard yet
    keyboard(320);
    expect(typing()).toBe(true);
  });

  it('covers a search field and editable text, not a checkbox', () => {
    keyboard(320);
    (document.getElementById('search') as HTMLElement).focus();
    expect(typing()).toBe(true);
    (document.getElementById('done') as HTMLElement).focus();
    expect(typing()).toBe(false);
    expect(typesText(document.getElementById('line'))).toBe(true);
  });

  it("is not set for the bar's own search: it must not hide the bar it lives in", () => {
    keyboard(320);
    (document.getElementById('bar-search') as HTMLElement).focus();
    expect(typing()).toBe(false);
  });

  it('goes when the keyboard is put away, though the field keeps the focus', () => {
    (document.getElementById('journal') as HTMLElement).focus();
    keyboard(320);
    expect(typing()).toBe(true);
    keyboard(0);
    expect(typing()).toBe(false);
    expect(document.activeElement?.id).toBe('journal');
  });

  it('goes when the field is left, and does not flicker moving between fields', async () => {
    keyboard(320);
    (document.getElementById('journal') as HTMLElement).focus();
    (document.getElementById('search') as HTMLElement).focus();
    expect(typing()).toBe(true);
    (document.activeElement as HTMLElement).blur();
    await tick();
    expect(typing()).toBe(false);
  });

  it('leaves nothing behind when stopped', () => {
    keyboard(320);
    (document.getElementById('journal') as HTMLElement).focus();
    stop();
    expect(typing()).toBe(false);
  });
});

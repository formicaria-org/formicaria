// The contract between the note panel and its editor (`handle.ts`), asked of the editor.
//
// What a test cannot see is layout: there is none here, so where a thing is on screen (`caretBox`,
// `centre`, `reveal`) is only required not to throw and to answer zeros. Those are checked in a
// browser and on the phone.
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import LiveEditor from './LiveEditor.svelte';
import type { EditorHandle } from './handle';
import { typeInto } from './testing';
import * as undo from '../undo.svelte';

// A line that ends in spaces, a tab, an empty line, and no newline at the end.
const AWKWARD = 'one\ntwo  \n\tthree\n\n- [ ] four';
// And every way a line can end: the editor keeps them (a text box would not).
const ENDINGS = 'one\r\ntwo\nthree\rfour';

for (const [name, Editor] of [['the editor', LiveEditor]] as const) {
  describe(name, () => {
    async function mount(value: string, props: Record<string, unknown> = {}) {
      const { component, container, rerender } = render(
        Editor as never,
        { value, ...props } as never,
      );
      await tick();
      return { ed: component as unknown as EditorHandle, container, rerender };
    }
    it('holds a note exactly as it was given', async () => {
      const { ed } = await mount(AWKWARD);
      expect(ed.text()).toBe(AWKWARD);
    });

    it('counts positions as the text does', async () => {
      const { ed } = await mount(AWKWARD);
      const at = AWKWARD.indexOf('three');
      ed.select(at, at + 5);
      expect(ed.selection()).toEqual({ from: at, to: at + 5 });
      ed.select(AWKWARD.length);
      expect(ed.selection()).toEqual({ from: AWKWARD.length, to: AWKWARD.length });
    });

    it('takes a change made outside it and keeps the rest', async () => {
      const { ed, rerender } = await mount('one two three');
      await rerender({ value: 'one **two** three' } as never);
      await tick();
      expect(ed.text()).toBe('one **two** three');
    });

    it('answers about the screen without a screen', async () => {
      const { ed } = await mount('some text');
      expect(ed.caretBox(3)).toMatchObject({ top: 0, left: 0 });
      expect(() => {
        ed.centre(3);
        ed.reveal(3);
        ed.reveal(3, true);
        ed.intoView();
        ed.box();
      }).not.toThrow();
    });

    it('is named for what it is, so it can be found', async () => {
      const { container } = await mount('x');
      expect(container.querySelector('[aria-label="note body (Markdown)"]')).not.toBeNull();
      expect(container.querySelector('.editor')).not.toBeNull();
    });
  });
}

describe('the editor that renders as you write', () => {
  it('links selected words to an address pasted over them, and pastes anything else as it is', async () => {
    const { component, container } = render(
      LiveEditor as never,
      {
        value: 'read the paper today',
      } as never,
    );
    await tick();
    const ed = component as unknown as EditorHandle;
    const content = container.querySelector<HTMLElement>('.cm-content')!;
    const paste = (text: string) => {
      const e = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(e, 'clipboardData', { value: { getData: () => text } });
      content.dispatchEvent(e);
      return e.defaultPrevented;
    };
    ed.select(5, 14);
    expect(paste('https://example.org/x')).toBe(true);
    expect(ed.text()).toBe('read [the paper](https://example.org/x) today');
    // The cursor is after the link, ready to go on writing.
    expect(ed.selection().from).toBe('read [the paper](https://example.org/x)'.length);
    // Anything that is not one address replaces the selection, as a paste always has.
    ed.select(0, 4);
    paste('not an address');
    expect(ed.text()).toBe('not an address [the paper](https://example.org/x) today');
  });

  /// What is on screen is not the text: the marks are hidden and the note is drawn finished. The
  /// text it holds is still every character of the note.
  it('draws a note finished and still holds it as written', async () => {
    const note = '## Title\n\na **bold** word\n\n- [ ] a task';
    const { component, container } = render(LiveEditor as never, { value: note } as never);
    await tick();
    const shown = [...container.querySelectorAll('.cm-line')].map((l) => l.textContent).join('\n');
    expect(shown).toBe('Title\n\na bold word\n\n a task');
    expect(container.querySelector('.cm-h2')).not.toBeNull();
    expect(container.querySelector('.cm-strong')?.textContent).toBe('bold');
    expect(container.querySelector('input.cm-task')).not.toBeNull();
    expect((component as unknown as EditorHandle).text()).toBe(note);
  });

  /// The text is the file, to the byte: a `\r` is a character on its line, kept where it was, and
  /// every position is still an index into the note's text.
  it('keeps every kind of line ending, and counts positions across them as the text does', async () => {
    const { component } = render(LiveEditor as never, { value: ENDINGS } as never);
    await tick();
    const ed = component as unknown as EditorHandle;
    expect(ed.text().replace(/\r/g, '␍')).toBe(ENDINGS.replace(/\r/g, '␍'));
    const at = ENDINGS.indexOf('four');
    ed.select(at, at + 4);
    expect(ed.selection()).toEqual({ from: at, to: at + 4 });
  });

  /// Dragging a selection's end across a line must not move the text under it: the marks of the
  /// lines it touches stay hidden, as they were when the selection began (2026-10-17).
  it('does not show the marks of the lines a selection runs across', async () => {
    const note = 'plain line\n\na **bold** word\n\n## Title';
    const { component, container } = render(LiveEditor as never, { value: note } as never);
    await tick();
    const ed = component as unknown as EditorHandle;
    ed.focus();
    ed.select(2); // a caret on the first line: its marks (none) are shown
    ed.select(2, note.length); // now the selection reaches the bold word and the heading
    await tick();
    const shown = [...container.querySelectorAll('.cm-line')].map((l) => l.textContent).join('\n');
    expect(shown).toBe('plain line\n\na bold word\n\nTitle');
    // A caret on the bold word's line shows that line as written.
    ed.select(note.indexOf('bold'));
    await tick();
    expect(container.querySelector('.cm-content')?.textContent).toContain('**bold**');
  });

  /// A phone has no Ctrl+Z: the ↶ menu undoes what is typed in the open editor, a change at a time,
  /// then goes on to the app's own steps (2026-10-17).
  it('lends its typing to the undo menu while it is written in', async () => {
    undo.clear();
    const { component, container, unmount } = render(
      LiveEditor as never,
      { value: 'first', undoLabel: 'typing in “Lab”' } as never,
    );
    await tick();
    const ed = component as unknown as EditorHandle;
    ed.focus();
    await tick();
    expect(undo.nextUndo()).toBeNull();
    typeInto(container.querySelector('.cm-content')!, 'second');
    await tick();
    expect(undo.nextUndo()?.label).toBe('typing in “Lab”');
    expect(await undo.undo()).toBe('typing in “Lab”');
    expect(ed.text()).toBe('first');
    expect(undo.nextUndo()).toBeNull();
    expect(await undo.redo()).toBe('typing in “Lab”');
    expect(ed.text()).toBe('second');
    unmount();
    // Closed, it lends nothing: the finished session is the note panel's step to record.
    expect(undo.nextUndo()).toBeNull();
  });
});

describe('pictures in the editor', () => {
  it('draws one with the renderer it is given, and leaves it as text without one', async () => {
    const value = 'a shot\n![shot](asset:sha256-ab12)\n';
    const paint = vi.fn(async (el: HTMLElement, md: string) => {
      el.textContent = `[drawn ${md}]`;
    });
    const drawn = render(LiveEditor as never, { value, embeds: paint } as never);
    await tick();
    await vi.waitFor(() =>
      expect(drawn.container.querySelector('.cm-embed')?.textContent).toBe(
        '[drawn ![shot](asset:sha256-ab12)]',
      ),
    );
    expect(paint).toHaveBeenCalledTimes(1);

    const plain = render(LiveEditor as never, { value } as never);
    await tick();
    expect(plain.container.querySelector('.cm-embed')).toBeNull();
    expect(plain.container.textContent).toContain('![shot](asset:sha256-ab12)');
  });
});

// One contract, two editors (`handle.ts`). Whatever the note panel may ask of an editor is asked
// here of both, so the second editor cannot quietly mean something different by the same call.
//
// What a test cannot see is layout: there is none here, so where a thing is on screen (`caretBox`,
// `centre`, `reveal`) is only required not to throw and to answer zeros. Those are checked in a
// browser and on the phone.
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeAll, describe, expect, it } from 'vitest';
import PlainEditor from './PlainEditor.svelte';
import LiveEditor from './LiveEditor.svelte';
import type { EditorHandle } from './handle';

beforeAll(() => {
  // CodeMirror measures text with ranges; jsdom has the method on elements and not on ranges.
  const none = () => ({ length: 0, item: () => null, [Symbol.iterator]: [][Symbol.iterator] });
  Range.prototype.getClientRects ??= none as never;
  Range.prototype.getBoundingClientRect ??= (() => new DOMRect()) as never;
});

// A line that ends in spaces, a tab, an empty line, and no newline at the end.
const AWKWARD = 'one\ntwo  \n\tthree\n\n- [ ] four';
// And every way a line can end. **Only the live editor is asked to keep these**: a textarea
// turns `\r\n` and `\r` into `\n` by itself (it is in the HTML standard), so the plain editor has
// always rewritten such a note's line endings the first time it is edited. Found by this test,
// 2026-10-11; `known-issues.md` has it.
const ENDINGS = 'one\r\ntwo\nthree\rfour';

for (const [name, Editor] of [
  ['the plain editor', PlainEditor],
  ['the editor that renders as you write', LiveEditor],
] as const) {
  describe(name, () => {
    async function mount(value: string, props: Record<string, unknown> = {}) {
      const { component, container, rerender } = render(
        Editor as never,
        { value, ...props } as never,
      );
      await tick();
      return { ed: component as unknown as EditorHandle, container, rerender };
    }
    /** The text the editor holds, read from the editor and not from what it was given. */
    const held = (container: HTMLElement) => {
      const ta = container.querySelector('textarea');
      if (ta) return ta.value;
      return (container.querySelector('.cm-content') as HTMLElement & { cmView?: unknown })
        ? [...container.querySelectorAll('.cm-line')].map((l) => l.textContent).join('\n')
        : '';
    };

    it('holds a note exactly as it was given', async () => {
      const { container } = await mount(AWKWARD);
      expect(held(container)).toBe(AWKWARD);
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
      const { container, rerender } = await mount('one two three');
      await rerender({ value: 'one **two** three' } as never);
      await tick();
      expect(held(container)).toBe('one **two** three');
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
  /// The text is the file, to the byte: a `\r` is a character on its line, kept where it was, and
  /// every position is still an index into the note's text.
  it('keeps every kind of line ending, and counts positions across them as the text does', async () => {
    const { component, container } = render(LiveEditor as never, { value: ENDINGS } as never);
    await tick();
    const ed = component as unknown as EditorHandle;
    const lines = [...container.querySelectorAll('.cm-line')].map((l) => l.textContent);
    expect(lines.join('\n').replace(/\r/g, '␍')).toBe(ENDINGS.replace(/\r/g, '␍'));
    const at = ENDINGS.indexOf('four');
    ed.select(at, at + 4);
    expect(ed.selection()).toEqual({ from: at, to: at + 4 });
  });
});

<script lang="ts">
  // The plain editor: a textarea over the note's exact text. It is the default and the fallback
  // (`decisions.md`, 2026-10-11), and everything it knows about a textarea stays in this file —
  // the panel speaks `EditorHandle` (`handle.ts`) and nothing else.
  import { caretXY } from '../caret';
  import type { EditorHandle } from './handle';

  let {
    value = $bindable(''),
    oninput,
    onkeydown,
    onselect,
    ondragover,
    ondrop,
    onblur,
  }: {
    value?: string;
    oninput?: () => void;
    onkeydown?: (e: KeyboardEvent) => void;
    /** The selection may have changed: a select, a mouse release or a key release. */
    onselect?: () => void;
    ondragover?: (e: DragEvent) => void;
    ondrop?: (e: DragEvent) => void;
    onblur?: () => void;
  } = $props();

  let el = $state<HTMLTextAreaElement | undefined>(undefined);

  export const selection: EditorHandle['selection'] = () => ({
    from: el?.selectionStart ?? 0,
    to: el?.selectionEnd ?? 0,
  });
  export const select: EditorHandle['select'] = (from, to = from) => {
    if (!el) return;
    el.selectionStart = from;
    el.selectionEnd = to;
  };
  export const focus: EditorHandle['focus'] = (opts) => el?.focus(opts);
  export const hasFocus: EditorHandle['hasFocus'] = () => !!el && document.activeElement === el;
  export const box: EditorHandle['box'] = () => el?.getBoundingClientRect() ?? new DOMRect();
  export const caretBox: EditorHandle['caretBox'] = (pos) => {
    if (!el) return { top: 0, left: 0, lineHeight: 0 };
    const r = el.getBoundingClientRect();
    const at = caretXY(el, pos);
    return { top: r.top + at.top, left: r.left + at.left, lineHeight: at.lineHeight };
  };
  /// Scroll so the caret's line is in the middle of the box. Near the top of a note there is
  /// nothing above to scroll, so the caret stays where the text puts it; near the end the bottom
  /// padding on touch (`.editor`, below) is what gives it room.
  export const centre: EditorHandle['centre'] = (pos) => {
    if (!el) return;
    const { top, lineHeight } = caretXY(el, pos);
    el.scrollTop += top + (lineHeight || 22) / 2 - el.clientHeight / 2;
  };
  /// Out of sight includes the last line of the box: a caret on the editor's bottom edge is
  /// measured as inside it and read as hidden, against the keyboard with no line below it.
  export const reveal: EditorHandle['reveal'] = (pos, always = false) => {
    if (!el) return;
    const { top, lineHeight } = caretXY(el, pos);
    const lh = lineHeight || 22;
    if (always || top < 0 || top + 2 * lh > el.clientHeight) centre(pos);
  };
  export const intoView: EditorHandle['intoView'] = () =>
    el?.scrollIntoView?.({ block: 'nearest' });
</script>

<textarea
  class="editor"
  bind:this={el}
  bind:value
  {oninput}
  {onkeydown}
  {onselect}
  onmouseup={onselect}
  onkeyup={onselect}
  {ondragover}
  {ondrop}
  {onblur}
  spellcheck="false"
  aria-label="note body (Markdown)"></textarea>

<style>
  .editor {
    width: 100%;
    min-height: 22rem;
    resize: vertical;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-5);
    border: none;
    background: transparent;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: 1.6;
  }
  /* **Room below the last line, on touch.** The caret is centred when the editor opens, and the
     last lines of a note can only reach the middle if there is something beneath them to scroll.
     About half the editor's own height is that something.

     **Sized from `--app-h`, never from `vh`.** A textarea cannot be shorter than its padding, and
     the editor has to shrink when the keyboard takes half the screen: a first version used `50vh`,
     which is taller than the editor is with the keyboard up, so the editor stopped shrinking and
     its lower half went under the keyboard. `--app-h` already has the keyboard taken out, and 14rem
     is about what the header, the quick row and the app's bar take from it.

     Not with a precise pointer, where a desktop editor ending in half a screen of blank would only
     look broken. */
  @media (pointer: coarse) {
    .editor {
      padding-bottom: max(var(--space-6), calc((var(--app-h) - 14rem) / 2));
    }
  }
  /* With the keyboard up the editor fits what is left rather than holding 22rem, which on a phone is
     taller than the room above the keyboard — the note would scroll as a whole and its last lines,
     the ones being typed, would be under the keyboard again. */
  :global(html[data-keyboard]) .editor {
    min-height: 0;
  }
  .editor:focus-visible {
    outline: none;
    box-shadow: inset 2px 0 0 var(--accent);
  }
</style>

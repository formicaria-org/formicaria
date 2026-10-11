<script lang="ts">
  // The live editor: CodeMirror over the note's exact text (`decisions.md`, 2026-10-11, *the
  // editor renders as you write, and the text is still the file*). The same `EditorHandle` as the
  // plain editor, so the panel cannot tell them apart.
  //
  // **The text is the file, to the byte.** CodeMirror would otherwise treat `\r\n`, `\r` and `\n`
  // alike and write them all back as one kind, and it counts every line break as one position —
  // so offsets into its document would stop matching offsets into the note's text, which is what
  // the whole panel works in. Declaring `\n` the only separator fixes both at once: a `\r` is then
  // an ordinary character on its line, kept where it was, and every position is a string index.
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import type { EditorHandle } from './handle';
  import { smallestChange } from './kind';

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
    onselect?: () => void;
    ondragover?: (e: DragEvent) => void;
    ondrop?: (e: DragEvent) => void;
    onblur?: () => void;
  } = $props();

  let host = $state<HTMLDivElement | undefined>(undefined);
  let view: EditorView | undefined;

  onMount(() => {
    view = new EditorView({
      parent: host!,
      state: EditorState.create({
        doc: value,
        extensions: [
          EditorState.lineSeparator.of('\n'),
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          EditorView.lineWrapping,
          EditorView.contentAttributes.of({
            'aria-label': 'note body (Markdown)',
            spellcheck: 'false',
          }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) {
              // What the editor holds is now the draft. Said before `oninput`, which reads it.
              value = u.state.doc.toString();
              oninput?.();
            }
            if (u.selectionSet || u.docChanged) onselect?.();
            if (u.focusChanged && !u.view.hasFocus) onblur?.();
          }),
          // The panel's handlers come first: the slash menu takes the arrows and Enter while it
          // is open, and a file dropped on the note is the panel's to take in. What they prevent
          // is not then also done by the editor.
          EditorView.domEventHandlers({
            keydown: (e) => {
              onkeydown?.(e);
              return e.defaultPrevented;
            },
            dragover: (e) => {
              ondragover?.(e);
              return e.defaultPrevented;
            },
            drop: (e) => {
              ondrop?.(e);
              return e.defaultPrevented;
            },
          }),
        ],
      }),
    });
    return () => view?.destroy();
  });

  // The panel changed the draft (a format button, a link, a picture, a merge that came back):
  // take it as the smallest edit that gets there.
  $effect(() => {
    const now = value;
    if (!view) return;
    const change = smallestChange(view.state.doc.toString(), now);
    if (change) view.dispatch({ changes: change });
  });

  const zeros = { top: 0, left: 0, lineHeight: 0 };
  export const selection: EditorHandle['selection'] = () => {
    const main = view?.state.selection.main;
    return { from: main?.from ?? 0, to: main?.to ?? 0 };
  };
  export const select: EditorHandle['select'] = (from, to = from) => {
    if (!view) return;
    // A position outside the text is the nearest one inside it, as a textarea treats it.
    const within = (n: number) => Math.max(0, Math.min(n, view!.state.doc.length));
    view.dispatch({ selection: { anchor: within(from), head: within(to) } });
  };
  export const focus: EditorHandle['focus'] = (opts) => {
    if (opts?.preventScroll) view?.contentDOM.focus({ preventScroll: true });
    else view?.focus();
  };
  export const hasFocus: EditorHandle['hasFocus'] = () => !!view?.hasFocus;
  export const box: EditorHandle['box'] = () => view?.dom.getBoundingClientRect() ?? new DOMRect();
  export const caretBox: EditorHandle['caretBox'] = (pos) => {
    if (!view) return zeros;
    const at = view.coordsAtPos(Math.min(pos, view.state.doc.length));
    return at ? { top: at.top, left: at.left, lineHeight: at.bottom - at.top } : zeros;
  };
  export const centre: EditorHandle['centre'] = (pos) => {
    if (!view) return;
    const at = Math.min(pos, view.state.doc.length);
    view.dispatch({ effects: EditorView.scrollIntoView(at, { y: 'center' }) });
  };
  export const reveal: EditorHandle['reveal'] = (pos, always = false) => {
    if (!view) return;
    if (always) return centre(pos);
    const at = view.coordsAtPos(Math.min(pos, view.state.doc.length));
    const seen = view.scrollDOM.getBoundingClientRect();
    // Out of sight includes the last line of the box, as in the plain editor: against the
    // keyboard there is no line below it.
    if (!at || at.top < seen.top || at.bottom + (at.bottom - at.top) > seen.bottom) centre(pos);
  };
  export const intoView: EditorHandle['intoView'] = () =>
    view?.dom.scrollIntoView?.({ block: 'nearest' });
</script>

<div class="editor live" bind:this={host}></div>

<style>
  .editor {
    width: 100%;
    min-height: 22rem;
    box-sizing: border-box;
    display: flex;
    color: var(--text);
  }
  .editor :global(.cm-editor) {
    flex: 1;
    min-width: 0;
    background: transparent;
    outline: none;
  }
  .editor :global(.cm-scroller) {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: 1.6;
    overflow: auto;
  }
  .editor :global(.cm-content) {
    padding: var(--space-4) var(--space-5);
    caret-color: var(--text);
  }
  .editor :global(.cm-line) {
    padding: 0;
  }
  .editor :global(.cm-selectionBackground),
  .editor :global(.cm-content ::selection) {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }
  /* Room below the last line on touch, and no fixed height with the keyboard up: the same two
     rules as the plain editor, for the same reasons (see `PlainEditor.svelte`). */
  @media (pointer: coarse) {
    .editor :global(.cm-content) {
      padding-bottom: max(var(--space-6), calc((var(--app-h) - 14rem) / 2));
    }
  }
  :global(html[data-keyboard]) .editor {
    min-height: 0;
  }
  .editor:focus-within {
    box-shadow: inset 2px 0 0 var(--accent);
  }
</style>

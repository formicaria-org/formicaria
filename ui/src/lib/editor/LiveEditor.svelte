<script lang="ts">
  // The editor: CodeMirror over the note's exact text, drawn as the finished note while it is
  // written (`decisions.md`, 2026-10-11, *the editor renders as you write, and the text is still
  // the file*). It is the only editor; the note panel speaks to it through `EditorHandle`.
  //
  // **The text is the file, to the byte.** CodeMirror would otherwise treat `\r\n`, `\r` and `\n`
  // alike and write them all back as one kind, and it counts every line break as one position —
  // so offsets into its document would stop matching offsets into the note's text, which is what
  // the whole panel works in. Declaring `\n` the only separator fixes both at once: a `\r` is then
  // an ordinary character on its line, kept where it was, and every position is a string index.
  import { onMount } from 'svelte';
  import { EditorState, Prec } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import type { EditorHandle } from './handle';
  import { smallestChange } from './change';
  import { livePreview } from './livePreview';
  import { linkOnPaste } from './paste';

  let {
    value = $bindable(''),
    oninput,
    onkeydown,
    onselect,
    ondragover,
    ondrop,
    onblur,
    onlink,
    label = 'note body (Markdown)',
  }: {
    value?: string;
    oninput?: () => void;
    onkeydown?: (e: KeyboardEvent) => void;
    onselect?: () => void;
    ondragover?: (e: DragEvent) => void;
    ondrop?: (e: DragEvent) => void;
    onblur?: () => void;
    /** A link drawn finished was pressed once: follow it. */
    onlink?: (href: string) => void;
    /** What the editor is called to assistive technology — a note, or a proposed note under review. */
    label?: string;
  } = $props();

  let host = $state<HTMLDivElement | undefined>(undefined);
  let view: EditorView | undefined;
  /** How long a second press may follow the first and still make it a double one. */
  const DOUBLE_MS = 280;
  let following: ReturnType<typeof setTimeout> | undefined;

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
          livePreview,
          EditorView.contentAttributes.of({
            'aria-label': label,
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
          // is not then also done by the editor. **`Prec.highest` is what makes "first" true**:
          // without it the editor's own keys ran before these, and Enter on the slash menu chose
          // nothing and started a new line (caught by the suite, 2026-10-11).
          Prec.highest(
            EditorView.domEventHandlers({
              // **A press on a finished link follows it; a double press edits it** (the owner,
              // 2026-10-11: a link that renders well could not be followed, because every press
              // in it meant "edit this"). The first press of a double one has to wait a moment to
              // find out which it is, which is why following is not instant.
              mousedown: (e, v) => {
                const link = (e.target as HTMLElement | null)?.closest<HTMLElement>('[data-href]');
                if (!link || e.button !== 0) return false;
                e.preventDefault();
                clearTimeout(following);
                if (e.detail === 1) {
                  const href = link.dataset.href!;
                  following = setTimeout(() => onlink?.(href), DOUBLE_MS);
                } else {
                  const at = v.posAtCoords({ x: e.clientX, y: e.clientY });
                  if (at !== null) v.dispatch({ selection: { anchor: at } });
                  v.focus();
                }
                return true;
              },
              // An address pasted over selected words links them (`paste.ts` says when).
              paste: (e, v) => {
                const main = v.state.selection.main;
                const link = linkOnPaste(
                  v.state.sliceDoc(main.from, main.to),
                  e.clipboardData?.getData('text/plain') ?? '',
                );
                if (link === null) return false;
                e.preventDefault();
                v.dispatch({
                  changes: { from: main.from, to: main.to, insert: link },
                  selection: { anchor: main.from + link.length },
                  userEvent: 'input.paste',
                });
                return true;
              },
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
          ),
        ],
      }),
    });
    return () => {
      clearTimeout(following);
      view?.destroy();
    };
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
  export const text: EditorHandle['text'] = () => view?.state.doc.toString() ?? value;
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
    // Out of sight includes the last line of the box: against the keyboard there is no line
    // below it, so a caret on the editor's bottom edge reads as hidden.
    if (!at || at.top < seen.top || at.bottom + (at.bottom - at.top) > seen.bottom) centre(pos);
  };
  export const intoView: EditorHandle['intoView'] = () =>
    view?.dom.scrollIntoView?.({ block: 'nearest' });
</script>

<div class="editor live" bind:this={host}></div>

<style>
  .editor {
    /* Fills what the format bar above it leaves of the note. */
    flex: 1 1 auto;
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
  /* It reads like the finished note, because it is drawn as one: the reading view's face and
     size, with code in the fixed-width face. */
  .editor :global(.cm-scroller) {
    font-family: var(--font-sans);
    font-size: var(--text-base);
    line-height: var(--lh-md);
    overflow: auto;
  }
  .editor :global(.cm-h1) {
    font-size: 1.6em;
    font-weight: 700;
    line-height: 1.3;
  }
  .editor :global(.cm-h2) {
    font-size: 1.35em;
    font-weight: 700;
    line-height: 1.3;
  }
  .editor :global(.cm-h3) {
    font-size: 1.15em;
    font-weight: 650;
  }
  .editor :global(.cm-h4),
  .editor :global(.cm-h5),
  .editor :global(.cm-h6) {
    font-weight: 650;
  }
  .editor :global(.cm-strong) {
    font-weight: 700;
  }
  .editor :global(.cm-em) {
    font-style: italic;
  }
  .editor :global(.cm-strike) {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .editor :global(.cm-code) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    padding: 0.05em 0.3em;
    border-radius: var(--radius-sm);
    background: var(--surface-hover);
  }
  .editor :global(.cm-highlight) {
    background: color-mix(in srgb, var(--tint-c) 35%, transparent);
    border-radius: 2px;
  }
  .editor :global(.cm-link) {
    color: var(--link);
    text-decoration: underline;
  }
  .editor :global(.cm-link[data-href]) {
    cursor: pointer;
  }
  .editor :global(.cm-quote) {
    padding-left: var(--space-3);
    border-left: 3px solid var(--border-strong);
    color: var(--text-muted);
  }
  .editor :global(.cm-codeblock) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--surface-hover);
    padding-inline: var(--space-2);
  }
  .editor :global(.cm-rule) {
    color: var(--text-subtle);
    border-bottom: 1px solid var(--border-strong);
  }
  .editor :global(.cm-bullet) {
    display: inline-block;
    width: 1ch;
    color: var(--text-muted);
  }
  .editor :global(.cm-task) {
    margin: 0 0.15em 0 0;
    vertical-align: middle;
    cursor: pointer;
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
  /* **Room below the last line, on touch.** The caret is centred when the editor opens, and the
     last lines of a note can only reach the middle if there is something beneath them to scroll.
     Sized from `--app-h`, never from `vh`: `--app-h` already has the keyboard taken out, so the
     editor still shrinks when the keyboard takes half the screen. And with the keyboard up the
     editor fits what is left instead of holding 22rem, which on a phone is taller than the room
     above the keyboard. */
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

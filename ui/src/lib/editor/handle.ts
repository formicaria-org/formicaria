/// What the note panel may ask of its editor.
///
/// **The panel owns the note; an editor only holds text and a selection, and says where things
/// are on screen** (`decisions.md`, 2026-10-11, *the editor renders as you write, and the text is
/// still the file*). The draft, the save debounce, the version guard, the slash menu, the format
/// bar and the quick row all stay in the panel and are written against this. There is one editor
/// (`LiveEditor.svelte`); the seam is kept because it is what stops the panel reaching into it,
/// and what a replacement would have to implement.
///
/// The text itself travels by `bind:value`, the way it always has: the panel changes the draft,
/// waits a tick for the editor to take it, then sets the selection.
///
/// Positions are character offsets into the text. Boxes are in **viewport** coordinates, because
/// what they anchor (the format bar, the slash menu) is positioned against the viewport. Where
/// there is no layout engine (the test suite) a box is zeros, never a throw.
export interface EditorHandle {
  /** The text the editor holds: the whole note, exactly. */
  text(): string;
  /** The selection; `from === to` is a bare caret. */
  selection(): { from: number; to: number };
  /** Select `from..to`, or put the caret at `from`. */
  select(from: number, to?: number): void;
  focus(opts?: { preventScroll?: boolean }): void;
  /** Is the keyboard focus in this editor? */
  hasFocus(): boolean;
  /** The line box of the character at `pos`. */
  caretBox(pos: number): { top: number; left: number; lineHeight: number };
  /** The editor's own box. */
  box(): DOMRect;
  /** Scroll the editor so `pos` is in the middle of what it shows. */
  centre(pos: number): void;
  /** Centre `pos` when it is out of sight — or regardless, when `always` is set. */
  reveal(pos: number, always?: boolean): void;
  /** Bring the editor itself into its pane, by its nearest edge. */
  intoView(): void;
}

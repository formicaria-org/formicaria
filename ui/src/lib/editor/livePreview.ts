/// Applies [`preview`](./preview.ts) to a CodeMirror editor: the pure list of what to draw becomes
/// decorations over the text. Nothing here decides anything; it only draws what `decide` says.
import { Decoration, EditorView, ViewPlugin, WidgetType } from '@codemirror/view';
import type { DecorationSet, ViewUpdate } from '@codemirror/view';
import { decide, read, type Reading } from './preview';

class Bullet extends WidgetType {
  toDOM() {
    const dot = document.createElement('span');
    dot.className = 'cm-bullet';
    dot.textContent = '•';
    return dot;
  }
  eq() {
    return true;
  }
}

/// A tick box in place of `[ ]`. Pressing it rewrites those three characters in the note, which
/// is the only thing a tick is.
class Tick extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly from: number,
  ) {
    super();
  }
  eq(other: Tick) {
    return other.checked === this.checked && other.from === this.from;
  }
  toDOM(view: EditorView) {
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.className = 'cm-task';
    box.checked = this.checked;
    box.setAttribute('aria-label', this.checked ? 'done, press to reopen' : 'press when finished');
    // On press, not on click: a click would first move the cursor to this line and show the
    // marks, taking the box away from under the pointer.
    box.addEventListener('mousedown', (e) => {
      e.preventDefault();
      view.dispatch({
        changes: { from: this.from, to: this.from + 3, insert: this.checked ? '[ ]' : '[x]' },
      });
    });
    return box;
  }
  ignoreEvent() {
    return false;
  }
}

const hide = Decoration.replace({});
const bullet = Decoration.replace({ widget: new Bullet() });

/// `at` is where the marks are shown, or -1 for nowhere: a note nobody is writing in is shown
/// finished throughout, with no line "the one being written".
function draw(reading: Reading, at: number): DecorationSet {
  const ranges = decide(reading, { from: at, to: at }).map((d) => {
    switch (d.kind) {
      case 'hide':
        return hide.range(d.from, d.to);
      case 'mark':
        return Decoration.mark({
          class: d.cls,
          ...(d.href ? { attributes: { 'data-href': d.href } } : {}),
        }).range(d.from, d.to);
      case 'line':
        return Decoration.line({ class: d.cls }).range(d.at);
      case 'bullet':
        return bullet.range(d.from, d.to);
      case 'task':
        return Decoration.replace({ widget: new Tick(d.checked, d.from) }).range(d.from, d.to);
    }
  });
  return Decoration.set(ranges, true);
}

/// The note is read again only when its text changes; a move of the cursor only changes what is
/// hidden.
///
/// **A selection does not change what is drawn** (owner, 2026-10-17: selecting on the phone was
/// "not good", and a stretch of chosen length could not be selected anywhere). The marks used to
/// come back on every line the selection touched, so dragging its end across a line put that line's
/// `**`, `#` or link address back, the text moved under the finger or the pointer, and the end
/// landed somewhere else. Now the marks follow the caret only: while a stretch is selected they stay
/// where they were when the selection began — nowhere, if the editor was not being written in.
export const livePreview = ViewPlugin.fromClass(
  class {
    reading: Reading;
    decorations: DecorationSet;
    /** Where the marks are shown: the caret's last place, or -1 for nowhere. */
    at = -1;
    constructor(view: EditorView) {
      this.reading = read(view.state.doc.toString());
      this.at = this.reveal(view, -1);
      this.decorations = draw(this.reading, this.at);
    }
    reveal(view: EditorView, was: number): number {
      const main = view.state.selection.main;
      if (!view.hasFocus) return -1;
      return main.empty ? main.head : was;
    }
    update(u: ViewUpdate) {
      if (u.docChanged) this.reading = read(u.state.doc.toString());
      const was = u.docChanged && this.at >= 0 ? u.changes.mapPos(this.at) : this.at;
      const at = this.reveal(u.view, was);
      if (u.docChanged || at !== this.at) {
        this.at = at;
        this.decorations = draw(this.reading, at);
      }
    }
  },
  { decorations: (p) => p.decorations },
);

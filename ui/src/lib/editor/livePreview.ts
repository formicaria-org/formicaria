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

function draw(reading: Reading, view: EditorView): DecorationSet {
  const main = view.state.selection.main;
  // A note nobody is writing in is shown finished throughout: no line is "the one being written".
  const sel = view.hasFocus ? { from: main.from, to: main.to } : { from: -1, to: -1 };
  const ranges = decide(reading, sel).map((d) => {
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
export const livePreview = ViewPlugin.fromClass(
  class {
    reading: Reading;
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.reading = read(view.state.doc.toString());
      this.decorations = draw(this.reading, view);
    }
    update(u: ViewUpdate) {
      if (u.docChanged) this.reading = read(u.state.doc.toString());
      if (u.docChanged || u.selectionSet || u.focusChanged)
        this.decorations = draw(this.reading, u.view);
    }
  },
  { decorations: (p) => p.decorations },
);

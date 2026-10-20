/// Applies [`preview`](./preview.ts) to a CodeMirror editor: the pure list of what to draw becomes
/// decorations over the text. Nothing here decides anything; it only draws what `decide` says.
import { Facet } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, WidgetType } from '@codemirror/view';
import type { DecorationSet, ViewUpdate } from '@codemirror/view';
import { decide, read, type Reading } from './preview';
import { drawMath } from '../render';

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

/// How an embed is drawn: the reading view's own `renderInto`, handed in by the note panel, so the
/// editor writes no HTML of its own and a picture looks the same in both. Without one (a test, a
/// proposal under review) an embed stays as its text.
export type DrawEmbed = (el: HTMLElement, markdown: string) => Promise<void>;
export const drawEmbed = Facet.define<DrawEmbed, DrawEmbed | null>({
  combine: (all) => all[0] ?? null,
});

/// A picture (or recording, or embedded note) in place of its `![…](…)`. The same text is the same
/// widget, so typing elsewhere in the note does not load it again.
class Embed extends WidgetType {
  constructor(
    readonly source: string,
    readonly paint: DrawEmbed,
  ) {
    super();
  }
  eq(other: Embed) {
    return other.source === this.source;
  }
  toDOM(view: EditorView) {
    const box = document.createElement('span');
    box.className = 'cm-embed';
    // A picture has no height until it has loaded; the editor is told when it has one, or its
    // idea of where the lines are would be wrong below it.
    void this.paint(box, this.source).then(() => {
      measure(view, box);
      for (const m of box.querySelectorAll('img, video'))
        m.addEventListener('load', () => measure(view, box), { once: true });
    });
    return box;
  }
}

/// Tell the editor a drawn thing has its size now — **only while it is still on screen**. Drawing
/// finishes later than the line it stands in (a picture loads, KaTeX is fetched), and by then the
/// note may be closed: measuring a gone editor ran its listeners against a panel already torn
/// down (Svelte's `derived_inert`), and in the suite it made the next test wait seconds.
function measure(view: EditorView, box: HTMLElement) {
  if (box.isConnected && view.dom.isConnected) view.requestMeasure();
}

/// A formula in place of its `$…$`, through the reading view's own KaTeX (`drawMath`).
class Formula extends WidgetType {
  constructor(
    readonly tex: string,
    readonly display: boolean,
    readonly src: string,
  ) {
    super();
  }
  eq(other: Formula) {
    return other.tex === this.tex && other.display === this.display;
  }
  toDOM(view: EditorView) {
    const box = document.createElement('span');
    box.className = this.display ? 'cm-math cm-math-display' : 'cm-math';
    void drawMath(box, { tex: this.tex, display: this.display, src: this.src }).then(() =>
      measure(view, box),
    );
    return box;
  }
}

const hide = Decoration.replace({});
const bullet = Decoration.replace({ widget: new Bullet() });

/// `at` is where the marks are shown, or -1 for nowhere: a note nobody is writing in is shown
/// finished throughout, with no line "the one being written".
function draw(reading: Reading, at: number, paint: DrawEmbed | null): DecorationSet {
  const decos = decide(reading, { from: at, to: at });
  const ranges = (paint ? decos : decos.filter((d) => d.kind !== 'embed')).map((d) => {
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
      case 'math':
        return Decoration.replace({
          widget: new Formula(d.tex, d.display, reading.text.slice(d.from, d.to)),
        }).range(d.from, d.to);
      case 'embed':
        return Decoration.replace({
          widget: new Embed(reading.text.slice(d.from, d.to), paint!),
        }).range(d.from, d.to);
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
      this.decorations = draw(this.reading, this.at, view.state.facet(drawEmbed));
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
        this.decorations = draw(this.reading, at, u.state.facet(drawEmbed));
      }
    }
  },
  { decorations: (p) => p.decorations },
);

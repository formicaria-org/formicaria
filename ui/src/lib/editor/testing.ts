// For tests that need the editor's text, or a change typed into it: CodeMirror is a content-editable
// view, not a text box, so `.value` and an `input` event do not reach it. These go through the view —
// the same change typing makes.
import { EditorView } from '@codemirror/view';

function viewOf(el: Element): EditorView {
  const view = EditorView.findFromDOM(el as HTMLElement);
  if (!view) throw new Error('not an editor');
  return view;
}

/** The editor's whole text. */
export function editorText(el: Element): string {
  return viewOf(el).state.doc.toString();
}

/** Replace the editor's whole text, as if typed. */
export function typeInto(el: Element, text: string): void {
  const view = viewOf(el);
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
}

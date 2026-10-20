/// Suite-wide test configuration.
///
/// **Why this exists: a 1-second default that is not about the code.** Testing Library's `findBy*`
/// helpers retry for `asyncUtilTimeout`, 1000 ms by default. Most files here mount the whole `App` —
/// jsdom, Svelte hydration, lazy panel imports — and with the files running in parallel, one extra
/// test file was enough to push a heavy mount past that second. The symptom was a test failing in
/// `App.features.test.ts` while the file that "caused" it touched nothing it uses (2026-07-31), which
/// sends you hunting for shared state that vitest already isolates per file.
///
/// So the timeout is set from what a loaded machine actually needs, not from a library default. It
/// bounds only *waiting*: a `findBy*` that will succeed is unaffected, and a genuine failure costs a
/// few extra seconds once. Deliberately not a `testTimeout` bump — that would hide a hang.
import { configure } from '@testing-library/svelte';
import { setFirstView } from './lib/panes';

// **Most of the suite was written against a first screen of Board.** Since 2026-10-18 the app opens
// on Home; the tests about Home set it back themselves (`App.home.test.ts`).
setFirstView('board');
import { EditorView } from '@codemirror/view';
import { afterEach } from 'vitest';
import { idle as navIdle, reset as resetNav } from './lib/nav.svelte';

// **Back's copy of the stack belongs to one test.** It settles a moment after a press, and a step
// back the browser was asked for cannot be called off — left alone it lands in the next test, on a
// page from before that test's app. So each test's history work is let finish before the next.
afterEach(async () => {
  await navIdle();
  resetNav();
});

configure({ asyncUtilTimeout: 5000 });

/// **jsdom ships no `matchMedia`, and that silently deleted the phone from the test suite.**
///
/// Every `(pointer: coarse)` branch in the app — the persistent format bar, the tap→move menu
/// path, the touch shells — is chosen by `window.matchMedia('(pointer: coarse)').matches`. With no
/// `matchMedia` at all, `platform.ts`'s guarded read answers `false` and those branches were
/// unreachable in CI for the entire life of the project. `App.svelte` says as much in a comment:
/// *"Verified at a narrow viewport and by `pointer: coarse`, NOT on a device."*
///
/// So the suite supplies one. It **defaults to a fine pointer**, which is what every existing test
/// already assumed, so nothing changes for them; `asPhone()` in `lib/harness.ts` flips it.
///
/// Deliberately not a `vi.fn()`: `clearMocks: true` in `vitest.config.ts` resets mock
/// implementations between tests, which would strip the stub out from under any test that set it
/// up in a `beforeAll`. A plain function survives.
let coarse = false;
/// A screen narrower than the layouts' breakpoints (`max-width` queries). Off by default, like the
/// fine pointer; a test of a phone's two-screen layout turns it on with `setNarrowScreen`.
let narrow = false;
export function setNarrowScreen(on: boolean): void {
  narrow = on;
}

/// Set by `asPhone()` / `asDesktop()`. Exported rather than reached through `window` so a test
/// that wants a finger without the rest of the phone can say exactly that.
export function setCoarsePointer(on: boolean): void {
  coarse = on;
}

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  configurable: true,
  value: (query: string): MediaQueryList =>
    ({
      // The app asks two things of it. `(pointer: coarse)` is the one that picks touch branches;
      // anything else (a width query, `prefers-reduced-motion`) answers false, which is jsdom's
      // own posture — it applies no CSS, so there is no viewport to report on honestly.
      matches: query.includes('pointer: coarse')
        ? coarse
        : query.includes('max-width')
          ? narrow
          : false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList,
});

/// **The editor measures text with ranges, and jsdom has that method on elements only.** Without
/// these two, mounting it throws from its first measurement.
Range.prototype.getClientRects ??= (() => ({
  length: 0,
  item: () => null,
  [Symbol.iterator]: [][Symbol.iterator],
})) as never;
Range.prototype.getBoundingClientRect ??= (() => new DOMRect()) as never;

/// **In a test, the editor answers as a text box does.**
///
/// The note's editor used to be a `<textarea>`, and some forty places in this suite drive it the
/// way a textarea is driven: set `value` and fire `input`, set `selectionStart`/`selectionEnd`
/// and fire `select`, read `value` back. The editor is now CodeMirror, whose text and selection
/// live in its own state and not on an element. Rather than rewrite forty call sites into its
/// vocabulary, the element they already find (`aria-label="note body (Markdown)"`) is given those
/// three properties here, each a view onto the real editor: setting one dispatches a real change
/// or a real selection, and reading one reads the real document. So the tests still exercise the
/// editor itself, by the same gestures.
///
/// On every `div`, because the editor's element is one; on any other `div` they answer as if
/// absent, which is what a `div` without them does.
const editorOf = (el: Element) => EditorView.findFromDOM(el as HTMLElement);
Object.defineProperties(HTMLDivElement.prototype, {
  value: {
    configurable: true,
    get(this: HTMLDivElement) {
      return editorOf(this)?.state.doc.toString();
    },
    set(this: HTMLDivElement, text: string) {
      const view = editorOf(this);
      if (!view) return;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text },
        selection: { anchor: text.length },
      });
    },
  },
  selectionStart: {
    configurable: true,
    get(this: HTMLDivElement) {
      return editorOf(this)?.state.selection.main.from;
    },
    set(this: HTMLDivElement, at: number) {
      const view = editorOf(this);
      if (!view) return;
      const head = Math.max(at, view.state.selection.main.to);
      view.dispatch({ selection: { anchor: at, head } });
    },
  },
  selectionEnd: {
    configurable: true,
    get(this: HTMLDivElement) {
      return editorOf(this)?.state.selection.main.to;
    },
    set(this: HTMLDivElement, at: number) {
      const view = editorOf(this);
      if (!view) return;
      const anchor = Math.min(view.state.selection.main.from, at);
      view.dispatch({ selection: { anchor, head: at } });
    },
  },
});

/// **jsdom ships no `Element.scrollIntoView`** (every browser has it). Settings scrolls to the
/// section a link asked for — the quiet-vault chip lands on Vaults — and in jsdom that call threw an
/// unhandled error that failed the run. A no-op: there is no layout here to scroll.
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = function () {};
}

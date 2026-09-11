// Where is the caret, in pixels? A textarea only exposes a character offset
// (`selectionStart`), and there is no browser API for the pixel position of one
// — so the standard trick: build a hidden div that is a typographic clone of the
// textarea, put the text up to the caret in it, and measure where a marker span
// lands. Because the clone wraps text identically, the span sits exactly where
// the caret does.
//
// Used to anchor the `/` insert menu to the caret instead of a fixed corner.

/** Styles that affect where text wraps and how tall a line is — the clone needs
 *  every one of them, or the mirror wraps differently and the answer is wrong. */
const MIRRORED = [
  'box-sizing',
  'width',
  'padding-top',
  'padding-right',
  'padding-bottom',
  'padding-left',
  'border-top-width',
  'border-right-width',
  'border-bottom-width',
  'border-left-width',
  'font-family',
  'font-size',
  'font-weight',
  'font-style',
  'font-variant',
  'letter-spacing',
  'line-height',
  'text-indent',
  'text-transform',
  'word-spacing',
  'tab-size',
] as const;

export type CaretPos = {
  /** Offset from the textarea's top-left, in CSS px, of the caret's line box. */
  top: number;
  left: number;
  /** The caret line's height — the caller offsets by this to sit *below* it. */
  lineHeight: number;
};

/**
 * The pixel position of character `index` within `el`, relative to `el`'s own
 * top-left corner and already adjusted for its scroll.
 *
 * Returns zeros where there is no layout engine (jsdom in the test suite), which
 * is why the caller must treat this as a hint and still clamp: a menu at 0,0 is
 * merely the old fixed-corner behavior, never a crash.
 */
export function caretXY(el: HTMLTextAreaElement, index: number): CaretPos {
  const style = window.getComputedStyle(el);
  const mirror = document.createElement('div');
  for (const prop of MIRRORED) mirror.style.setProperty(prop, style.getPropertyValue(prop));
  // Wrap exactly like a textarea, and take the mirror out of the visual flow so
  // measuring it never paints or reflows anything the user can see.
  mirror.style.whiteSpace = 'pre-wrap';
  mirror.style.overflowWrap = 'break-word';
  mirror.style.position = 'absolute';
  mirror.style.visibility = 'hidden';
  mirror.style.top = '0';
  mirror.style.left = '-9999px';
  mirror.style.height = 'auto';

  mirror.textContent = el.value.slice(0, index);
  const marker = document.createElement('span');
  // A zero-width space, so the span has a line box to measure even at the very
  // end of the text or on an empty trailing line.
  marker.textContent = '​';
  mirror.appendChild(marker);

  document.body.appendChild(mirror);
  const top = marker.offsetTop - el.scrollTop;
  const left = marker.offsetLeft - el.scrollLeft;
  const lineHeight = marker.offsetHeight || parseFloat(style.lineHeight) || 0;
  mirror.remove();

  return { top, left, lineHeight: Number.isFinite(lineHeight) ? lineHeight : 0 };
}

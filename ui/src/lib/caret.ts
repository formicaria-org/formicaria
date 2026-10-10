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

/** The properties a phone's text size setting scales. See [`undoubled`]. */
const ZOOMED = ['font-size', 'line-height'] as const;

/**
 * What to set on the mirror so that it *renders* at `want`, given that setting `want` rendered
 * as `got`. `null` when they already agree, which is every desktop browser.
 *
 * **Why they can disagree.** Android's WebView applies the phone's *Font size* setting as a text
 * zoom, and it is already inside what `getComputedStyle` reports: a textarea styled at 14px on a
 * phone set to 1.3x reports 18.2px. Copy that number onto the mirror and the zoom is applied to it
 * **again**, so the mirror draws at 23.7px, wraps sooner and stands taller than the textarea it is
 * supposed to be a clone of. Every line adds to the error, so a caret far down a long note was
 * measured hundreds of pixels below where it was: the editor scrolled past it, "leaving it even
 * outside the view" (the owner's phone, 2026-10-11), while every desktop measurement was exact.
 *
 * So the mirror is corrected against itself rather than against a guess at the zoom: set the value,
 * read back what it became, and if it grew by some factor, ask for that much less.
 */
export function undoubled(want: number, got: number): number | null {
  if (!Number.isFinite(want) || !Number.isFinite(got) || want <= 0 || got <= 0) return null;
  if (Math.abs(got - want) < 0.01) return null;
  return (want * want) / got;
}

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
  for (const prop of ZOOMED) {
    const want = parseFloat(style.getPropertyValue(prop));
    const got = parseFloat(window.getComputedStyle(mirror).getPropertyValue(prop));
    const fixed = undoubled(want, got);
    if (fixed !== null) mirror.style.setProperty(prop, `${fixed}px`);
  }
  const left = marker.offsetLeft - el.scrollLeft;
  const lineHeight = parseFloat(style.lineHeight) || marker.offsetHeight || 0;
  // The twin's answer when there is one: it is the textarea's own layout, not a likeness of it.
  const top = (twinTop(el, index, lineHeight) ?? marker.offsetTop) - el.scrollTop;
  mirror.remove();

  return { top, left, lineHeight: Number.isFinite(lineHeight) ? lineHeight : 0 };
}

/**
 * How far down the text the caret's line starts, measured in **a second copy of the textarea
 * itself** — or `null` where there is no layout to measure (jsdom).
 *
 * The div mirror above is a likeness: it is told the textarea's computed styles and trusted to lay
 * text out the same way. That trust failed on a phone (see `undoubled`), and a likeness can fail
 * again for a reason nobody has met yet — a font the engine substitutes in form controls only, a
 * text setting applied to one kind of element and not the other. So for the number that matters
 * most, the vertical position, nothing is copied: the twin is the same element with the same
 * classes in the same parent, so whatever the engine does to the original it does to the twin.
 * Its height is forced to nothing, which makes `scrollHeight` exactly the height of its text.
 *
 * The text goes on to the end of the word the caret is in, so that word wraps where it does in
 * the original rather than fitting on the line above for want of its last letters.
 */
function twinTop(el: HTMLTextAreaElement, index: number, lineHeight: number): number | null {
  const parent = el.parentElement;
  if (!parent || !lineHeight) return null;
  const twin = el.cloneNode(false) as HTMLTextAreaElement;
  twin.removeAttribute('id');
  twin.removeAttribute('aria-label');
  twin.setAttribute('aria-hidden', 'true');
  twin.tabIndex = -1;
  twin.readOnly = true;
  const rest = /^\S*/.exec(el.value.slice(index));
  twin.value = el.value.slice(0, index) + (rest ? rest[0] : '');
  const t = twin.style;
  t.position = 'absolute';
  t.visibility = 'hidden';
  t.pointerEvents = 'none';
  t.left = '0';
  t.top = '0';
  t.width = `${el.offsetWidth}px`;
  t.height = '0';
  t.minHeight = '0';
  t.maxHeight = 'none';
  t.paddingBottom = '0';
  t.overflow = 'hidden';
  t.flex = 'none';
  t.resize = 'none';
  parent.appendChild(twin);
  const height = twin.scrollHeight; // padding-top + the text, with nothing beneath it
  twin.remove();
  return height > 0 ? height - lineHeight : null;
}

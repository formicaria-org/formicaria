// **How a selection can come in, when that is a real choice** (the chooser by a selection; owner,
// 2026-11-17). Its own small module: the selection script that loads on every page imports it, and
// must not carry the Markdown conversion (`bring.ts`) into every page.
import { onlyFormula } from './formula';

/** How a piece comes in: a formula as LaTeX or its picture; a video or recording whole, as its
 *  sound, or as a link. */
export type How = 'latex' | 'picture' | 'video' | 'mp3' | 'link';

/** **How it can come in, when that is a real choice** (owner, 2026-11-17: *"if there is a video the
 *  tool can propose full video or just sound, if there is a formula it can propose a LaTeX code …
 *  or port the image with the formula"*). The first is the one offered by default. */
export function offersFor(s: { html: string }): { how: How; label: string }[] {
  const f = s.html.trim() ? onlyFormula(s.html) : null;
  if (f) {
    const pic = /<img\b[^>]*\bsrc=/i.test(s.html);
    return pic
      ? [
          { how: 'latex', label: 'As a formula (LaTeX)' },
          { how: 'picture', label: 'As a picture' },
        ]
      : [];
  }
  const d = new DOMParser().parseFromString(`<body>${s.html}</body>`, 'text/html').body;
  const media = d.querySelectorAll('video, audio');
  const words = (d.textContent ?? '').trim();
  if (media.length === 1 && !words) {
    const video = media[0].tagName === 'VIDEO';
    return [
      { how: 'video', label: video ? 'The whole video' : 'The recording' },
      { how: 'mp3', label: 'Only the sound, as MP3' },
      { how: 'link', label: 'A link to it' },
    ];
  }
  return [];
}

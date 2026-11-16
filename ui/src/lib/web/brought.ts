// **What the phone's page screen brought in, as note text** (`workspace-design` §12, the phone's
// browser, phase A). The same code as the computer's browser (`bring.ts`): a passage quoted with its
// formatting and a link to it on the page, a table as a table, a link as a link, the page as a link;
// an image as a link (the phone keeps no copy of it yet). Every piece ends with the page's exact
// address and when.
import type { Brought } from '../share.svelte';
import { classify, imageLinked, linkPiece, pagePiece, textPiece, type Piece } from './bring';

export function broughtText(b: Brought, when: string): string {
  const page = { url: b.pageUrl, title: b.pageTitle };
  const base = { pageUrl: b.pageUrl, pageTitle: b.pageTitle, when };
  let p: Piece;
  if (b.kind === 'image') p = { ...base, kind: 'image', src: b.src };
  else if (b.kind === 'link') p = { ...base, kind: 'link', src: b.src, alt: b.src };
  else if (b.kind === 'page') p = { ...base, kind: 'page' };
  else p = { ...classify(b.html ?? '', b.text ?? '', page), when };
  switch (p.kind) {
    case 'text':
      return textPiece(p);
    case 'image':
      return imageLinked(p);
    case 'link':
      return linkPiece(p);
    default:
      return `${pagePiece(p)}, ${when}`;
  }
}

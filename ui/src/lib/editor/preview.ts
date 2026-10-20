/// What the editor that renders as you write draws differently from the text, and where.
///
/// **Pure, and it never changes the text.** It reads the note's Markdown and answers with a list
/// of decorations: this stretch is bold, these two characters are hidden, this line is a heading.
/// The editor applies them over the exact text, which stays the file (`decisions.md`, 2026-10-11).
///
/// **The marks come back where you are writing.** A construct on a line the selection touches is
/// shown as written, `**` and all, so what is being typed is never hidden from the person typing
/// it. Everywhere else the marks are hidden and only the result is seen.
///
/// Parsed with `@lezer/markdown`, the grammar CodeMirror's own Markdown support uses, with the
/// GitHub extensions (tick boxes, strikethrough, tables, bare links).
import { parser as base, GFM } from '@lezer/markdown';
import type { SyntaxNode, Tree } from '@lezer/common';
import { TEXT_TOKENS } from '../render-vocab';
import { findMath } from '../render';

const parser = base.configure(GFM);

export type Deco =
  /** Not drawn at all. */
  | { kind: 'hide'; from: number; to: number }
  /** Drawn with a class. `href` is where a link leads, given only while the link is drawn
   *  finished: then a press follows it, and a double press edits it. */
  | { kind: 'mark'; from: number; to: number; cls: string; href?: string }
  /** The whole line starting at `at` gets a class. */
  | { kind: 'line'; at: number; cls: string }
  /** A list's `-` drawn as a bullet. */
  | { kind: 'bullet'; from: number; to: number }
  /** A `[ ]` or `[x]` drawn as a box that can be pressed. */
  | { kind: 'task'; from: number; to: number; checked: boolean }
  /** `![words](address)` — a picture, a recording, an embedded note — drawn as the reading view
   *  draws it, in place of its text. */
  | { kind: 'embed'; from: number; to: number }
  /** A formula, `$…$` or `$$…$$`, drawn as the reading view draws it. */
  | { kind: 'math'; from: number; to: number; tex: string; display: boolean };

/// The note, read once. Kept apart from [`decide`] so a move of the cursor, which changes what is
/// hidden and not what the note says, does not read it again.
export interface Reading {
  text: string;
  tree: Tree;
  /** Where each line starts. */
  lines: number[];
}

export function read(text: string): Reading {
  const lines = [0];
  for (let i = text.indexOf('\n'); i !== -1; i = text.indexOf('\n', i + 1)) lines.push(i + 1);
  return { text, tree: parser.parse(text), lines };
}

/** The index of the line holding `pos`. */
function lineOf(lines: number[], pos: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (lines[mid] <= pos) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

const HEADING = /^ATXHeading([1-6])$/;
const INLINE: Record<string, string> = {
  StrongEmphasis: 'cm-strong',
  Emphasis: 'cm-em',
  Strikethrough: 'cm-strike',
  InlineCode: 'cm-code',
};
const INLINE_MARKS = new Set(['EmphasisMark', 'StrikethroughMark', 'CodeMark']);

export function decide(r: Reading, sel: { from: number; to: number }): Deco[] {
  const { text, tree, lines } = r;
  const out: Deco[] = [];
  const first = lineOf(lines, Math.min(sel.from, sel.to));
  const last = lineOf(lines, Math.max(sel.from, sel.to));
  /** Is the selection on any line this stretch is on? Then it is shown as written. A selection
   *  before the text (`from < 0`) is nobody writing anywhere: everything is drawn finished. */
  const nobody = sel.from < 0;
  const writing = (from: number, to: number) =>
    !nobody && lineOf(lines, from) <= last && lineOf(lines, Math.max(from, to - 1)) >= first;
  const eachLine = (from: number, to: number, cls: string) => {
    for (let l = lineOf(lines, from); l <= lineOf(lines, Math.max(from, to - 1)); l++)
      out.push({ kind: 'line', at: lines[l], cls });
  };
  /** Stretches where `==` means nothing: code, inline or fenced. */
  const code: [number, number][] = [];

  tree.iterate({
    enter(n) {
      const name = n.name;
      const heading = HEADING.exec(name);
      if (heading) {
        out.push({ kind: 'line', at: lines[lineOf(lines, n.from)], cls: `cm-h${heading[1]}` });
        const mark = n.node.getChild('HeaderMark');
        if (mark && mark.from === n.from && !writing(n.from, n.to)) {
          // The `#`s and the space after them.
          let to = mark.to;
          while (text[to] === ' ' || text[to] === '\t') to++;
          out.push({ kind: 'hide', from: mark.from, to });
        }
        return;
      }
      if (INLINE[name]) {
        out.push({ kind: 'mark', from: n.from, to: n.to, cls: INLINE[name] });
        if (name === 'InlineCode') code.push([n.from, n.to]);
        if (!writing(n.from, n.to)) {
          for (let c = n.node.firstChild; c; c = c.nextSibling)
            if (INLINE_MARKS.has(c.name)) out.push({ kind: 'hide', from: c.from, to: c.to });
        }
        return;
      }
      if (name === 'Link') return link(n.node);
      if (name === 'Image') {
        if (!writing(n.from, n.to)) out.push({ kind: 'embed', from: n.from, to: n.to });
        return false;
      }
      if (name === 'FencedCode' || name === 'CodeBlock') {
        code.push([n.from, n.to]);
        eachLine(n.from, n.to, 'cm-codeblock');
        return false;
      }
      if (name === 'Blockquote') {
        eachLine(n.from, n.to, 'cm-quote');
        return;
      }
      if (name === 'QuoteMark') {
        if (!writing(n.from, n.to)) {
          const to = text[n.to] === ' ' ? n.to + 1 : n.to;
          out.push({ kind: 'hide', from: n.from, to });
        }
        return;
      }
      if (name === 'HorizontalRule') {
        out.push({ kind: 'line', at: lines[lineOf(lines, n.from)], cls: 'cm-rule' });
        return;
      }
      if (name === 'ListMark') return listMark(n.node);
      if (name === 'TaskMarker' && !writing(n.from, n.to)) {
        out.push({
          kind: 'task',
          from: n.from,
          to: n.to,
          checked: /x/i.test(text.slice(n.from, n.to)),
        });
      }
      return;
    },
  });

  /// A link shows its words and hides its address: `[`, and everything from `]` to the end.
  /// Only the inline kind, `[words](address)`; a reference link has nothing to hide.
  function link(n: SyntaxNode) {
    const marks = n.getChildren('LinkMark');
    if (!n.getChild('URL') || marks.length < 4) return;
    const [open, close] = marks;
    const url = n.getChild('URL')!;
    const being = writing(n.from, n.to);
    out.push({
      kind: 'mark',
      from: open.to,
      to: close.from,
      cls: 'cm-link',
      // On the line being written a press in its words is somebody correcting them.
      ...(being ? {} : { href: text.slice(url.from, url.to) }),
    });
    if (being) return;
    out.push({ kind: 'hide', from: open.from, to: open.to });
    out.push({ kind: 'hide', from: close.from, to: n.to });
  }

  /// A bullet's `-` becomes a dot; before a tick box it goes, and the box stands in for it. A
  /// number in a numbered list is left as written: it is already what it should look like.
  function listMark(n: SyntaxNode) {
    if (n.parent?.parent?.name !== 'BulletList' || writing(n.from, n.to)) return;
    const isTask = /^\s\[[ xX]\]\s/.test(text.slice(n.to, n.to + 5));
    if (isTask) out.push({ kind: 'hide', from: n.from, to: n.to + 1 });
    else out.push({ kind: 'bullet', from: n.from, to: n.to });
  }

  // `==highlight==` is this app's own mark, not in the grammar. Not inside code, and not across
  // lines; shown as written on the line being written, like everything else.
  // A single `=` inside is text (`a = b` in a highlighted sentence, 2026-10-20); only `==` ends it.
  for (const m of text.matchAll(/==(?=\S)((?:[^=\n]|=(?!=))*?\S)==/g)) {
    const from = m.index!;
    const to = from + m[0].length;
    if (code.some(([a, b]) => from < b && to > a)) continue;
    out.push({ kind: 'mark', from, to, cls: 'cm-highlight' });
    if (!writing(from, to)) {
      out.push({ kind: 'hide', from, to: from + 2 });
      out.push({ kind: 'hide', from: to - 2, to });
    }
  }
  // `[words]{.token}` is this app's coloured text: a named intent, never a hex. Drawn in that
  // colour with the brackets and the name hidden; an unknown name is left as written, exactly as
  // the reading view leaves it.
  for (const m of text.matchAll(/\[([^\]\n]+)\]\{\.([a-z]+)\}/g)) {
    if (!(TEXT_TOKENS as readonly string[]).includes(m[2])) continue;
    const from = m.index!;
    const to = from + m[0].length;
    if (code.some(([a, b]) => from < b && to > a)) continue;
    const words = { from: from + 1, to: from + 1 + m[1].length };
    out.push({ kind: 'mark', from: words.from, to: words.to, cls: `tk-${m[2]}` });
    if (!writing(from, to)) {
      out.push({ kind: 'hide', from, to: words.from });
      out.push({ kind: 'hide', from: words.to, to });
    }
  }
  // **Formulas** (2026-10-20), off the line being written, as pictures are. One on a line of its
  // own is a centred block, as in the reading view; one that runs over several lines stays as
  // written, since the editor cannot draw across a line's end. Nothing else is drawn inside one:
  // a `*` or `_` there is maths, not italics.
  const formulas = findMath(text).filter((m) => !text.slice(m.from, m.to).includes('\n'));
  if (!formulas.length) return out;
  const inside = (d: Deco) =>
    'from' in d && formulas.some((m) => d.from < m.to && d.to > m.from && d.kind !== 'mark');
  const kept = out.filter((d) => !inside(d));
  for (const m of formulas) {
    if (!writing(m.from, m.to))
      kept.push({ kind: 'math', from: m.from, to: m.to, tex: m.tex, display: m.display });
  }
  return kept;
}

/** [`read`] and [`decide`] in one step. */
export const preview = (text: string, sel: { from: number; to: number }) => decide(read(text), sel);

import { describe, expect, it } from 'vitest';
import { preview, type Deco } from './preview';

/** What a reader sees: the text with the hidden stretches taken out, a dot for a bullet and a
 *  box for a tick box. The cursor is at `at` (by default somewhere out of the way). */
function seen(text: string, at = text.length + 0) {
  const decos = preview(text + '\n\nelsewhere', { from: text.length + 5, to: text.length + 5 });
  return draw(
    text + '\n\nelsewhere',
    at === text.length ? decos : preview(text + '\n\nelsewhere', { from: at, to: at }),
  ).replace(/\n\nelsewhere$/, '');
}
function draw(text: string, decos: Deco[]) {
  const swaps = decos
    .flatMap((d) =>
      d.kind === 'hide'
        ? [{ from: d.from, to: d.to, with: '' }]
        : d.kind === 'bullet'
          ? [{ from: d.from, to: d.to, with: '•' }]
          : d.kind === 'task'
            ? [{ from: d.from, to: d.to, with: d.checked ? '☑' : '☐' }]
            : [],
    )
    .sort((a, b) => b.from - a.from);
  let out = text;
  for (const s of swaps) out = out.slice(0, s.from) + s.with + out.slice(s.to);
  return out;
}
const classes = (text: string, at = 10_000) =>
  preview(text + '\n\nelsewhere', {
    from: Math.min(at, text.length + 5),
    to: Math.min(at, text.length + 5),
  })
    .flatMap((d) => (d.kind === 'mark' || d.kind === 'line' ? [d.cls] : []))
    .sort();

describe('what is drawn while you write', () => {
  it('shows a heading without its #s, at its size', () => {
    expect(seen('## Results')).toBe('Results');
    expect(classes('## Results')).toContain('cm-h2');
    expect(classes('###### Small')).toContain('cm-h6');
  });

  it('shows bold, italic, struck, code and highlighted text without their marks', () => {
    expect(seen('a **bold** b *it* c ~~gone~~ d `code` e ==lit== f')).toBe(
      'a bold b it c gone d code e lit f',
    );
    expect(classes('a **bold** b *it* c ~~gone~~ d `code` e ==lit== f')).toEqual([
      'cm-code',
      'cm-em',
      'cm-highlight',
      'cm-strike',
      'cm-strong',
    ]);
  });

  it('shows a link as its words', () => {
    expect(seen('see [the paper](https://example.org/x) now')).toBe('see the paper now');
    expect(classes('see [the paper](https://example.org/x) now')).toEqual(['cm-link']);
  });

  it('shows coloured text in its colour, without the brackets and the name', () => {
    expect(seen('a [careful]{.warn} word')).toBe('a careful word');
    expect(classes('a [careful]{.warn} word')).toEqual(['tk-warn']);
    // A name the app does not know is not a colour: left exactly as written.
    expect(seen('a [word]{.sparkly} here')).toBe('a [word]{.sparkly} here');
    // And on the line being written, the whole thing is in view.
    expect(seen('a [careful]{.warn} word', 4)).toBe('a [careful]{.warn} word');
    expect(seen('`[code]{.warn}`')).toBe('[code]{.warn}');
  });

  /// A finished link can be followed; one on the line being written is being corrected.
  it('says where a finished link leads, and not while it is being written', () => {
    const text = 'see [the paper](note:01ABC) now\n\nelsewhere';
    const href = (at: number) =>
      preview(text, { from: at, to: at }).flatMap((d) =>
        d.kind === 'mark' && d.href ? [d.href] : [],
      );
    expect(href(text.length)).toEqual(['note:01ABC']);
    expect(href(6)).toEqual([]);
  });

  it('draws a bullet as a dot and a tick box as a box, and leaves numbers alone', () => {
    expect(seen('- one\n- two')).toBe('• one\n• two');
    expect(seen('- [ ] open\n- [x] closed')).toBe('☐ open\n☑ closed');
    expect(seen('1. first\n2. second')).toBe('1. first\n2. second');
  });

  it('shows a quote without its >, and marks code blocks and rules by line', () => {
    expect(seen('> said\n> again')).toBe('said\nagain');
    expect(classes('> said')).toContain('cm-quote');
    expect(classes('```\ncode ==not lit==\n```')).toEqual([
      'cm-codeblock',
      'cm-codeblock',
      'cm-codeblock',
    ]);
    expect(classes('---')).toContain('cm-rule');
  });

  /// The one rule that makes it an editor: what you are writing is never hidden from you.
  it('shows the marks again on the line the cursor is on, and only there', () => {
    const text = '## Title\n\na **bold** word\n\n- item';
    const onBold = text.indexOf('bold');
    expect(seen(text, onBold)).toBe('Title\n\na **bold** word\n\n• item');
    expect(seen(text, 3)).toBe('## Title\n\na bold word\n\n• item');
    expect(seen(text, text.length - 1)).toBe('Title\n\na bold word\n\n- item');
    // Styled either way: the line being written still looks like what it is.
    expect(classes(text, onBold)).toContain('cm-strong');
  });

  it('draws everything finished when nobody is writing', () => {
    const text = '# First line\n\n**bold**';
    const decos = preview(text, { from: -1, to: -1 });
    expect(draw(text, decos)).toBe('First line\n\nbold');
  });

  it('never proposes hiding anything it does not understand', () => {
    for (const text of [
      'plain words, nothing to do',
      '![a picture](asset:sha256-abc)',
      '$$x = 1$$',
      '[[not a link]] and a [reference][1]',
      'a = b == c, and 2 == 2',
      '',
    ])
      expect(seen(text)).toBe(text);
  });

  it('only ever names stretches inside the text, in order within themselves', () => {
    const text = '# T\n\n**a** [b](c) `d` ==e==\n\n- [ ] f\n> g\n';
    for (const d of preview(text, { from: 0, to: 0 })) {
      if (d.kind === 'line') expect(d.at).toBeLessThanOrEqual(text.length);
      else {
        expect(d.from).toBeGreaterThanOrEqual(0);
        expect(d.to).toBeLessThanOrEqual(text.length);
        expect(d.from).toBeLessThan(d.to);
      }
    }
  });
});

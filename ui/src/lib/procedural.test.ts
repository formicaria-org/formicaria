// The phone's procedural hide rules (`ui/page/procedural.ts`), on invented pages, in the JSON form
// Brave's engine gives them.
import { describe, expect, it } from 'vitest';
import { apply, select, type Rule } from '../../page/procedural';

function page(html: string): Document {
  return new DOMParser().parseFromString(`<!doctype html><body>${html}</body>`, 'text/html');
}
const css = (arg: string) => ({ type: 'css-selector', arg });

describe('procedural hide rules', () => {
  it('hides a box by the words in it, and leaves the rest', () => {
    const d = page('<div class="c">Sponsored: buy</div><div class="c">An article</div>');
    const n = apply([{ selector: [css('div.c'), { type: 'has-text', arg: 'Sponsored' }] }], d);
    expect(n).toBe(1);
    const [ad, story] = [...d.querySelectorAll('div.c')] as HTMLElement[];
    expect(ad.style.getPropertyValue('display')).toBe('none');
    expect(ad.style.getPropertyPriority('display')).toBe('important');
    expect(story.style.display).toBe('');
  });

  it('reads a regular expression, goes up to the box around, and continues with CSS', () => {
    const d = page(
      '<section id="s1"><article><span class="lbl">Promoted</span><p>x</p></article></section>' +
        '<section id="s2"><article><span class="lbl">News</span></article></section>',
    );
    const up = select(
      [css('span.lbl'), { type: 'has-text', arg: '/^promo/i' }, { type: 'upward', arg: '2' }],
      d,
    );
    expect(up.map((e) => e.id)).toEqual(['s1']);
    const closest = select(
      [css('span.lbl'), { type: 'has-text', arg: 'Promoted' }, { type: 'upward', arg: 'section' }],
      d,
    );
    expect(closest.map((e) => e.id)).toEqual(['s1']);
    const inner = select([css('article'), { type: 'has-text', arg: 'Promoted' }, css('> p')], d);
    expect(inner.map((e) => e.tagName)).toEqual(['P']);
  });

  it('removes, restyles, and takes attributes and classes off', () => {
    const d = page(
      '<b class="x ad">1</b><i class="y" onclick="go()">2</i><u class="z">3</u><s class="w">4</s>',
    );
    const rules: Rule[] = [
      { selector: [css('.x')], action: { type: 'remove' } },
      { selector: [css('.y')], action: { type: 'remove-attr', arg: 'onclick' } },
      { selector: [css('.z')], action: { type: 'style', arg: 'opacity: 0' } },
      { selector: [css('.w')], action: { type: 'remove-class', arg: 'w' } },
    ];
    expect(apply(rules, d)).toBe(4);
    expect(d.querySelector('.x')).toBeNull();
    expect(d.querySelector('.y')?.hasAttribute('onclick')).toBe(false);
    expect(d.querySelector('.z')?.getAttribute('style')).toContain('opacity: 0');
    expect(d.querySelectorAll('.w').length).toBe(0);
  });

  it('matches attributes and the page address, and knows nothing it does not know', () => {
    const d = page('<a data-ad="yes">1</a><a data-ad="no">2</a>');
    expect(select([css('a'), { type: 'matches-attr', arg: 'data-ad="yes"' }], d).length).toBe(1);
    expect(select([css('a'), { type: 'matches-path', arg: '/shop' }], d, '/shop/item').length).toBe(
      2,
    );
    expect(select([css('a'), { type: 'matches-path', arg: '/shop' }], d, '/news').length).toBe(0);
    expect(select([css('a'), { type: 'min-text-length', arg: '2' }], d).length).toBe(0);
    expect(select([css('a'), { type: 'some-new-operator', arg: 'x' }], d)).toEqual([]);
    // A broken selector reaches nothing.
    expect(select([css('a[[[')], d)).toEqual([]);
  });
});

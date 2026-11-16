import { describe, expect, it } from 'vitest';
import { broughtText } from './brought';

const page = {
  pageUrl: 'https://example.org/walks/hills',
  pageTitle: 'A walk in the hills',
  note: '',
};

describe('what the phone page screen brought in', () => {
  it('a selection is quoted with its passage link and the time', () => {
    const md = broughtText(
      {
        ...page,
        kind: 'selection',
        html: '<p>The path <b>climbs</b>.</p>',
        text: 'The path climbs.',
      },
      '2026-11-16 12:00',
    );
    expect(md).toContain('> The path **climbs**.');
    expect(md).toContain(
      '(https://example.org/walks/hills#:~:text=The%20path%20climbs.), 2026-11-16 12:00',
    );
  });

  it('an image, a link, the page', () => {
    expect(
      broughtText({ ...page, kind: 'image', src: 'https://example.org/a.jpg' }, 'T'),
    ).toContain('[Image](https://example.org/a.jpg)');
    expect(broughtText({ ...page, kind: 'link', src: 'https://example.org/b' }, 'T')).toContain(
      '[https://example.org/b](https://example.org/b)',
    );
    expect(broughtText({ ...page, kind: 'page' }, 'T')).toBe(
      '[A walk in the hills](https://example.org/walks/hills), T',
    );
  });
});

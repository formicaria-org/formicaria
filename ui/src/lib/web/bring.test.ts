import { describe, expect, it } from 'vitest';
import {
  atMoment,
  classify,
  fileName,
  imageKept,
  imageLinked,
  linkPiece,
  mediaLinked,
  streamed,
  textPiece,
} from './bring';

const page = { url: 'https://example.org/walks/hills', title: 'A walk in the hills' };

describe('what a selection is', () => {
  it('tells one image, one link, media and text apart', () => {
    expect(classify('<img src="/a.jpg" alt="A ridge">', '', page)).toMatchObject({
      kind: 'image',
      src: 'https://example.org/a.jpg',
      alt: 'A ridge',
    });
    expect(classify('<a href="/next">Next walk</a>', 'Next walk', page)).toMatchObject({
      kind: 'link',
      src: 'https://example.org/next',
    });
    expect(classify('<video src="v.mp4"></video>', '', page)).toMatchObject({ kind: 'video' });
    // Firefox counts an image's description as the selection's text: still an image.
    expect(classify('<img src="/a.jpg" alt="A ridge">', 'A ridge', page).kind).toBe('image');
    expect(classify('<audio><source src="s.mp3"></audio>', '', page)).toMatchObject({
      kind: 'audio',
      src: 'https://example.org/walks/s.mp3',
    });
    expect(classify('<p>The path <b>climbs</b>.</p>', 'The path climbs.', page).kind).toBe('text');
    expect(classify('<p>See <a href="/x">this</a> walk</p>', 'See this walk', page).kind).toBe(
      'text',
    );
    expect(classify('', '', page).kind).toBe('page');
  });
});

describe('what it becomes in the note', () => {
  it('quotes text with its formatting and a link back to the passage', () => {
    const md = textPiece({
      ...classify('<p>The path <b>climbs</b> gently.</p>', 'The path climbs gently.', page),
    });
    expect(md).toContain('> The path **climbs** gently.');
    expect(md).toContain('> — [A walk in the hills](https://example.org/walks/hills#:~:text=');
  });

  it('keeps a table a table, not a quote', () => {
    const html = '<table><tr><th>Day</th><th>Km</th></tr><tr><td>Mon</td><td>12</td></tr></table>';
    const md = textPiece(classify(html, 'Day Km Mon 12', page));
    expect(md).toContain('| Day | Km |\n| --- | --- |\n| Mon | 12 |');
    expect(md.startsWith('>')).toBe(false);
  });

  it('keeps code as a code block', () => {
    const md = textPiece(classify('<pre><code>let a = 1;</code></pre>', 'let a = 1;', page));
    expect(md).toContain('```\nlet a = 1;\n```');
  });

  it('an image kept or linked, a link, always with the page', () => {
    const img = classify('<img src="/a.jpg" alt="A ridge">', '', page);
    expect(imageKept(img, 'sha256-ab')).toBe(
      "![A ridge](asset:sha256-ab)\n\n— [A walk in the hills](https://example.org/walks/hills) · [the image's address](https://example.org/a.jpg)",
    );
    expect(imageLinked(img)).toContain('[Image: A ridge](https://example.org/a.jpg)');
    expect(linkPiece(classify('<a href="/next">Next walk</a>', 'Next walk', page))).toBe(
      '[Next walk](https://example.org/next) — [A walk in the hills](https://example.org/walks/hills)',
    );
  });

  it('a stream becomes the page at the moment it was playing', () => {
    expect(streamed('blob:https://www.youtube.com/abc')).toBe(true);
    expect(streamed('https://a.org/v.mp4')).toBe(false);
    expect(atMoment('https://www.youtube.com/watch?v=x', 92.4)).toBe(
      'https://www.youtube.com/watch?v=x&t=92s',
    );
    expect(atMoment('https://vimeo.com/1', 61)).toBe('https://vimeo.com/1#t=61s');
    expect(atMoment('https://a.org/p', 61)).toBe('https://a.org/p');
    const md = mediaLinked({
      kind: 'video',
      pageUrl: 'https://www.youtube.com/watch?v=x',
      pageTitle: 'Ants',
      src: 'blob:x',
      time: 92,
    });
    expect(md).toBe('▶ [Ants at 1:32](https://www.youtube.com/watch?v=x&t=92s)');
  });

  it('says when it was brought in, beside where from', () => {
    const p = { ...classify('<a href="/n">Next</a>', 'Next', page), when: '2026-11-16 14:32' };
    expect(linkPiece(p)).toBe(
      '[Next](https://example.org/n) — [A walk in the hills](https://example.org/walks/hills), 2026-11-16 14:32',
    );
  });

  it('names a kept file after its address', () => {
    expect(fileName('https://a.org/media/song%201.mp3?x=1', 'audio')).toBe('song 1.mp3');
    expect(fileName('https://a.org/watch', 'video')).toBe('video');
  });
});

// A page kept from formicaria's own browser, as a note (`clip.ts`). Every page here is invented.
import { describe, expect, it } from 'vitest';
import { ingredientLines } from '../food/recipeText';
import { buildClip, citationBibtex, kindOf, textFragment, type PageFacts } from './clip';

const page = (over: Partial<PageFacts>): PageFacts => ({
  url: 'https://example.org/a',
  title: 'A page',
  jsonld: [],
  meta: {},
  ...over,
});

describe('what a page is', () => {
  it('reads what the page publishes for machines, before anything else', () => {
    expect(kindOf(page({ jsonld: [{ '@type': 'Recipe', name: 'Soup' }] }))).toBe('recipe');
    expect(kindOf(page({ jsonld: [{ '@graph': [{ '@type': ['Thing', 'Recipe'] }] }] }))).toBe(
      'recipe',
    );
    expect(kindOf(page({ meta: { citation_title: ['On things'] } }))).toBe('paper');
    expect(kindOf(page({ meta: { 'og:type': ['video.other'] } }))).toBe('video');
    expect(kindOf(page({ meta: { 'og:type': ['music.song'] } }))).toBe('song');
    expect(kindOf(page({ jsonld: [{ '@type': 'MusicAlbum' }] }))).toBe('song');
  });

  it('knows the common video and music pages by their address', () => {
    expect(kindOf(page({ url: 'https://www.youtube.com/watch?v=abc' }))).toBe('video');
    expect(kindOf(page({ url: 'https://youtu.be/abc' }))).toBe('video');
    expect(kindOf(page({ url: 'https://artist.bandcamp.com/album/first' }))).toBe('song');
  });

  it('is an article when it says nothing', () => {
    expect(kindOf(page({}))).toBe('article');
    expect(kindOf(page({ url: 'https://www.youtube.com/@channel' }))).toBe('article');
  });
});

describe('a highlight is a link back to the passage', () => {
  it('uses the Text Fragments form, dashes and commas escaped', () => {
    expect(textFragment('https://e.org/p#top', 'a well-known, plain phrase')).toBe(
      'https://e.org/p#:~:text=a%20well%2Dknown%2C%20plain%20phrase',
    );
  });
  it('anchors a long passage by its first and last words', () => {
    const long = 'one two three four five six seven eight nine ten eleven twelve';
    expect(textFragment('https://e.org/p', long)).toBe(
      'https://e.org/p#:~:text=one%20two%20three%20four%20five,eight%20nine%20ten%20eleven%20twelve',
    );
  });
});

describe('a paper', () => {
  const paper = page({
    url: 'https://journal.example/article/7',
    meta: {
      citation_title: ['Notes on {the} Engine'],
      citation_author: ['Lovelace, Ada', 'Babbage, Charles'],
      citation_publication_date: ['1843/09/01'],
      citation_journal_title: ['Scientific Memoirs'],
      citation_doi: ['10.1000/xyz123'],
      citation_firstpage: ['666'],
      citation_lastpage: ['731'],
      citation_abstract: ['What the engine might do.'],
    },
  });

  it('becomes the BibTeX entry create_paper reads', () => {
    const bib = citationBibtex(paper)!;
    expect(bib).toMatch(/^@article\{lovelace1843,/);
    expect(bib).toContain('title = {Notes on the Engine}');
    expect(bib).toContain('author = {Lovelace, Ada and Babbage, Charles}');
    expect(bib).toContain('journal = {Scientific Memoirs}');
    expect(bib).toContain('pages = {666--731}');
    expect(bib).toContain('doi = {10.1000/xyz123}');
  });

  it('keeps its abstract, the credit, and where it came from', () => {
    const d = buildClip(paper, '2026-11-15');
    expect(d.kind).toBe('paper');
    expect(d.title).toBe('Notes on {the} Engine');
    expect(d.bibtex).toBeTruthy();
    expect(d.body).toContain('What the engine might do.');
    expect(d.body).toContain(
      'Saved from <https://journal.example/article/7> (Scientific Memoirs) on 2026-11-15.',
    );
    expect(d.props).toMatchObject({ source_url: paper.url, saved: '2026-11-15' });
  });

  it('cites the date it was read', () => {
    expect(citationBibtex(paper, '2026-11-16')).toContain('urldate = {2026-11-16}');
  });
});

describe('the exact address a page was saved from', () => {
  it('is shown in full, with the time, and kept as it was visited', () => {
    const f = page({ url: 'https://news.example/a/1?id=7&utm_source=x' });
    const d = buildClip(f, '2026-11-16', null, '14:32');
    expect(d.body).toContain('Saved from <https://news.example/a/1?id=7&utm_source=x>');
    expect(d.body).toContain('on 2026-11-16 at 14:32.');
    expect(d.props.source_url).toBe('https://news.example/a/1?id=7&utm_source=x');
    expect(d.props.canonical_url).toBeUndefined();
  });

  it('adds the address to cite when the page declares another', () => {
    const f = page({
      url: 'https://m.news.example/a/1?ref=feed',
      canonical: 'https://news.example/a/1',
    });
    const d = buildClip(f, '2026-11-16');
    expect(d.props.canonical_url).toBe('https://news.example/a/1');
    expect(d.body).toContain('Address to cite: <https://news.example/a/1>');
    const same = page({
      url: 'https://news.example/a/1#top',
      canonical: 'https://news.example/a/1/',
    });
    expect(buildClip(same, '2026-11-16').props.canonical_url).toBeUndefined();
  });

  it("keeps a web app's route in the link back to a passage", () => {
    expect(textFragment('https://app.example/#/doc/5', 'the plan')).toBe(
      'https://app.example/#/doc/5:~:text=the%20plan',
    );
    expect(textFragment('https://app.example/#!/doc/5:~:text=old', 'new')).toBe(
      'https://app.example/#!/doc/5:~:text=new',
    );
  });

  it('is a conference paper when the page names a conference', () => {
    const conf = page({ meta: { citation_title: ['T'], citation_conference_title: ['Conf'] } });
    expect(citationBibtex(conf)).toMatch(/^@inproceedings\{paper,[\s\S]*booktitle = \{Conf\}/);
  });
});

describe('a recipe', () => {
  it('has the shape every recipe has, so its lines can be matched in Food', () => {
    const d = buildClip(
      page({ url: 'https://ricette.example/x', jsonld: [{ '@type': 'Recipe' }] }),
      '2026-11-15',
      {
        title: 'Pasta e ceci',
        url: 'https://ricette.example/x',
        site: 'Ricette',
        ingredients: ['200 g pasta', '400 g ceci'],
        steps: ['Cook.'],
        servings: 4,
      },
    );
    expect(d.kind).toBe('recipe');
    expect(d.tags).toEqual(['recipe', 'web']);
    expect(d.props).toMatchObject({ servings: 4, from: 'https://ricette.example/x' });
    expect(typeof d.props.recipe).toBe('string');
    expect(ingredientLines(d.body)).toEqual(['200 g pasta', '400 g ceci']);
  });
});

describe('an article, a video, a song', () => {
  it('an article keeps its text and the selected passage first', () => {
    const d = buildClip(
      page({
        title: 'Hills',
        selection: 'The path climbs gently.',
        article: { byline: 'A. Walker', markdown: '# Hills\n\nThe path climbs gently.' },
        meta: { 'og:site_name': ['Walks'] },
      }),
      '2026-11-15',
    );
    expect(d.kind).toBe('article');
    expect(d.tags).toEqual(['web', 'shelf:Web']);
    expect(d.body.startsWith('> The path climbs gently.')).toBe(true);
    expect(d.body).toContain('#:~:text=The%20path%20climbs%20gently.');
    expect(d.body).toContain('# Hills');
    expect(d.props).toMatchObject({ site: 'Walks', author: 'A. Walker' });
  });

  it('a video or a song keeps the link, never the media', () => {
    const v = buildClip(
      page({ url: 'https://www.youtube.com/watch?v=abc', meta: { 'og:title': ['A talk'] } }),
      '2026-11-15',
    );
    expect(v.kind).toBe('video');
    expect(v.title).toBe('A talk');
    expect(v.body).toContain('[Watch on youtube.com](https://www.youtube.com/watch?v=abc)');
    expect(v.tags).toContain('video');
    // A loose meta description is not trusted for a video; the item's own is.
    const withMeta = buildClip(
      page({
        url: 'https://www.youtube.com/watch?v=abc',
        meta: { 'og:description': ['another video entirely'] },
        jsonld: [{ '@type': 'VideoObject', description: 'Its own words.' }],
      }),
      '2026-11-15',
    );
    expect(withMeta.body).toContain('Its own words.');
    expect(withMeta.body).not.toContain('another video entirely');
    const s = buildClip(
      page({ url: 'https://artist.bandcamp.com/track/one', meta: { 'og:site_name': ['Artist'] } }),
      '2026-11-15',
    );
    expect(s.kind).toBe('song');
    expect(s.body).toContain('[Listen on Artist](https://artist.bandcamp.com/track/one)');
  });
});

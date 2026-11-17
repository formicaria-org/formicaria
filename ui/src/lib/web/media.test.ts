// **What a page holds that can be kept** (`media.ts`): the embedded players (a news article's YouTube
// video, by its watch address), the page's own video and audio, what it was seen fetching, and the
// page itself last; each once.
import { expect, it } from 'vitest';
import { findMedia, playerOf, wikimediaFile } from './media';

const doc = (html: string) => new DOMParser().parseFromString(html, 'text/html');

it('finds a news article’s embedded YouTube video, by its watch address', () => {
  const d = doc(`<title>Ants march on</title><article><p>Story</p>
    <iframe title="Leafcutters" src="https://www.youtube-nocookie.com/embed/AbCdEf12345?rel=0"></iframe>
    <iframe src="https://ads.example/frame"></iframe></article>`);
  const c = findMedia(d, 'https://news.example/ants');
  expect(c[0]).toEqual({
    url: 'https://www.youtube.com/watch?v=AbCdEf12345',
    kind: 'player',
    label: 'YouTube video: Leafcutters',
  });
  expect(c.at(-1)).toEqual({
    url: 'https://news.example/ants',
    kind: 'page',
    label: 'The page itself: Ants march on',
  });
  expect(c.some((x) => x.url.includes('ads.example'))).toBe(false);
});

it('finds the page’s own media, its streams and what it fetched, each once', () => {
  const d = doc(`<video src="/v/clip.mp4"><source src="/v/clip.mp4"></video>
    <audio><source src="https://cdn.example/a/ep1.mp3"></audio>`);
  const c = findMedia(d, 'https://site.example/p', [
    'https://cdn.example/hls/main.m3u8',
    'https://cdn.example/a/ep1.mp3',
    'https://cdn.example/x.js',
  ]);
  expect(c.map((x) => [x.kind, x.url])).toEqual([
    ['video', 'https://site.example/v/clip.mp4'],
    ['audio', 'https://cdn.example/a/ep1.mp3'],
    ['stream', 'https://cdn.example/hls/main.m3u8'],
    ['page', 'https://site.example/p'],
  ]);
});

it('knows the common players', () => {
  expect(playerOf('https://player.vimeo.com/video/76979871')?.url).toBe(
    'https://vimeo.com/76979871',
  );
  expect(playerOf('https://w.soundcloud.com/player/?url=x')?.site).toBe('SoundCloud');
  expect(playerOf('https://example.org/embed/1')).toBeNull();
});

it('a video site’s own page is offered as its video', () => {
  const d = doc(
    '<title>Leafcutters at night - YouTube</title><video src="blob:https://www.youtube.com/x"></video>',
  );
  const c = findMedia(d, 'https://www.youtube.com/watch?v=AbCdEf12345');
  expect(c).toEqual([
    {
      url: 'https://www.youtube.com/watch?v=AbCdEf12345',
      kind: 'player',
      label: 'YouTube video: Leafcutters at night',
    },
  ]);
});

// **Wikipedia's recordings** (found by `ci/usability-check.py` on a live article, 2026-11-17): its
// player script removes the `<source>` addresses from the page, leaving the file's page in `resource`.
it('finds every recording of a Wikipedia article, as its player left them', () => {
  const player = (file: string) =>
    `<span class="mw-tmh-player audio mw-file-element"><audio preload="none" data-mw-tmh="" resource="https://en.wikipedia.org/wiki/File:${file}" disabled="disabled"></audio><a class="mw-tmh-play" href="https://en.wikipedia.org/wiki/File:${file}" role="button"></a></span>`;
  const d = doc(`<title>A fugue - Wikipedia</title><p>Recordings:</p>
    ${player('Toccata_et_Fugue_BWV565.ogg')}${player('Second_%28organ%29.ogg')}${player('Third.mp3')}
    <a href="https://en.wikipedia.org/wiki/File:Third.mp3">the third, again</a>`);
  const found = findMedia(d, 'https://en.wikipedia.org/wiki/A_fugue');
  expect(found.map((c) => [c.kind, c.label])).toEqual([
    ['audio', 'Audio: Toccata_et_Fugue_BWV565.ogg'],
    ['audio', 'Audio: Second_(organ).ogg'],
    ['audio', 'Audio: Third.mp3'],
    ['page', 'The page itself: A fugue - Wikipedia'],
  ]);
  expect(found[1].url).toBe('https://commons.wikimedia.org/wiki/File:Second_%28organ%29.ogg');
});

it("offers a Commons file page's video as itself", () => {
  const d = doc(`<title>File:Ant Bed With Ants.webm - Wikimedia Commons</title>
    <span class="mw-tmh-player video"><video></video></span>`);
  const found = findMedia(d, 'https://commons.wikimedia.org/wiki/File:Ant_Bed_With_Ants.webm');
  expect(found[0]).toEqual({
    url: 'https://commons.wikimedia.org/wiki/File:Ant_Bed_With_Ants.webm',
    kind: 'video',
    label: 'Video: Ant_Bed_With_Ants.webm',
  });
  // Pictures are not media to fetch.
  expect(wikimediaFile('https://en.wikipedia.org/wiki/File:Ant.jpg')).toBeNull();
});

// **What a page holds that can be kept** (`media.ts`): the embedded players (a news article's YouTube
// video, by its watch address), the page's own video and audio, what it was seen fetching, and the
// page itself last; each once.
import { expect, it } from 'vitest';
import { findMedia, playerOf } from './media';

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

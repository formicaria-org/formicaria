// **A saved page is one self-contained file** (`inline.ts`): pictures, styles and fonts inside;
// video and audio never; within limits; what cannot be had keeps its address.
import { expect, it } from 'vitest';
import { inlinePage, bestOf, toBase64, type Got } from './inline';

const b64 = (s: string) => toBase64(new TextEncoder().encode(s));
const site: Record<string, Got> = {
  'https://news.example/img/ant-1200.jpg': { type: 'image/jpeg', base64: b64('JPEG-BIG') },
  'https://news.example/img/ant.jpg': { type: 'image/jpeg', base64: b64('JPEG') },
  'https://news.example/css/site.css': {
    type: 'text/css',
    base64: b64(
      '@import "print.css"; body{background:url(../img/bg.png)} @font-face{src:url("/f/a.woff2")}',
    ),
  },
  'https://news.example/css/print.css': { type: 'text/css', base64: b64('h1{color:red}') },
  'https://news.example/img/bg.png': { type: 'image/png', base64: b64('PNG') },
  'https://news.example/f/a.woff2': { type: 'font/woff2', base64: b64('WOFF') },
};
const asked: string[] = [];
const get = async (u: string) => {
  asked.push(u);
  return site[u] ?? null;
};
const PAGE = `<!doctype html><html><head><base href="https://news.example/story/">
<link rel="stylesheet" href="/css/site.css"><link rel="preconnect" href="https://cdn.example">
</head><body><h1>Ants</h1>
<img srcset="/img/ant.jpg 600w, /img/ant-1200.jpg 1200w" src="/img/ant.jpg" loading="lazy">
<img data-src="/img/missing.jpg" src="data:image/gif;base64,R0lGOD">
<video src="clip.mp4" poster="/img/ant.jpg"></video>
<div style="background-image:url('/img/bg.png')"></div></body></html>`;

it('puts pictures, style sheets, their imports, fonts and backgrounds inside', async () => {
  const r = await inlinePage(PAGE, 'https://news.example/story/', get);
  const doc = new DOMParser().parseFromString(r.html, 'text/html');
  const [big, missing] = Array.from(doc.querySelectorAll('img'));
  expect(big.getAttribute('src')).toBe(`data:image/jpeg;base64,${b64('JPEG-BIG')}`);
  expect(big.hasAttribute('srcset') || big.hasAttribute('loading')).toBe(false);
  // A lazy loader's real address is the one wanted; not to be had, it keeps its address.
  expect(missing.getAttribute('src')).toBe('https://news.example/img/missing.jpg');
  expect(doc.querySelector('link[rel=stylesheet]')).toBeNull();
  const style = doc.querySelector('style')!.textContent!;
  expect(style).toContain('h1{color:red}');
  expect(style).toContain(`url("data:image/png;base64,${b64('PNG')}")`);
  expect(style).toContain(`url("data:font/woff2;base64,${b64('WOFF')}")`);
  expect(doc.querySelector('div')!.getAttribute('style')).toContain('data:image/png');
  expect(doc.querySelector('link[rel=preconnect]')).toBeNull();
  expect(r.kept).toBe(1);
});

it('never puts video or audio inside: they keep their address', async () => {
  asked.length = 0;
  const r = await inlinePage(PAGE, 'https://news.example/story/', get);
  const v = new DOMParser().parseFromString(r.html, 'text/html').querySelector('video')!;
  expect(v.getAttribute('src')).toBe('https://news.example/story/clip.mp4');
  expect(asked).not.toContain('https://news.example/story/clip.mp4');
});

it('stays within its limits: past them, the rest keep their addresses', async () => {
  const r = await inlinePage(PAGE, 'https://news.example/story/', get, { each: 5, total: 7 });
  const doc = new DOMParser().parseFromString(r.html, 'text/html');
  // JPEG-BIG is 8 bytes, over `each`.
  expect(doc.querySelector('img')!.getAttribute('src')).toBe(
    'https://news.example/img/ant-1200.jpg',
  );
});

it('picks the widest picture of a srcset', () => {
  expect(bestOf('a.jpg 1x, b.jpg 2x')).toBe('b.jpg');
  expect(bestOf('s.jpg 320w, m.jpg 640w, l.jpg 1280w')).toBe('l.jpg');
  expect(bestOf('only.jpg')).toBe('only.jpg');
});

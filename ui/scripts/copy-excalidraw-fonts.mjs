// Self-host Excalidraw's fonts, so drawing never calls the internet.
//
// By default Excalidraw downloads its fonts from a CDN (esm.run). For a local-first tool
// whose README says "nothing phones home", that is simply false — opening a whiteboard
// would reach out. `window.EXCALIDRAW_ASSET_PATH` (set in index.html) points it at "/",
// and it then requests `./fonts/<Family>/<file>.woff2`, which is what this copies into
// `public/` for Vite to emit and `fm-serve/build.rs` to bake into the binary.
//
// Copied from the installed package at build time rather than committed: a checked-in
// copy silently goes stale the moment @excalidraw/excalidraw is bumped, and stale fonts
// are the kind of drift nobody notices.
//
// Xiaolai is skipped. It is 13 MB of CJK glyphs — 97% of the font payload — against
// ~390 KB for everything else, including Excalifont, the hand-drawn face the whole look
// depends on. Chinese/Japanese text in a whiteboard falls back to a system font; that is
// a documented trade, not a silent one. Drop it from SKIP to bundle it.
import { cp, mkdir, rm, readdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const SKIP = new Set(['Xiaolai']);

// Resolve the package's real entry rather than its package.json: the package's `exports`
// map deliberately does not expose ./package.json, so resolving that throws. The entry is
// `<pkg>/dist/prod/index.js`, and the fonts sit beside it.
const require = createRequire(import.meta.url);
const src = join(dirname(require.resolve('@excalidraw/excalidraw')), 'fonts');
const dest = join(import.meta.dirname, '..', 'public', 'fonts');

await rm(dest, { recursive: true, force: true });
await mkdir(dest, { recursive: true });

let copied = 0, skipped = [];
for (const family of await readdir(src)) {
  if (SKIP.has(family)) { skipped.push(family); continue; }
  await cp(join(src, family), join(dest, family), { recursive: true });
  copied++;
}
console.log(`excalidraw fonts: ${copied} families self-hosted, skipped ${skipped.join(', ') || 'none'}`);

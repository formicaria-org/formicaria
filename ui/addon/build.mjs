// Build formicaria's browser add-on into `dist/addon/formicaria.xpi` (`decisions.md` 2026-11-15).
//
// Run after `vite build` (the `build` script), so the `.xpi` lands in `ui/dist` and `fm-serve`'s
// build bakes it into the binary with the rest of the UI. Each script is bundled on its own, as a
// self-contained file — a content script cannot load modules — with Readability, Turndown and the
// shared `src/lib/web/clip.ts` inside. The `.xpi` is a plain zip, written here without a dependency.
import { build } from 'vite';
import { mkdirSync, readFileSync, rmSync, writeFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, '..', 'dist', 'addon');
const stage = join(out, 'stage');
rmSync(out, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });

for (const name of ['content', 'background', 'popup']) {
  await build({
    configFile: false,
    publicDir: false,
    logLevel: 'warn',
    define: { 'process.env.NODE_ENV': '"production"' },
    build: {
      outDir: stage,
      emptyOutDir: false,
      minify: true,
      target: 'firefox128',
      lib: {
        entry: join(here, 'src', `${name}.ts`),
        formats: ['iife'],
        name: `formicaria_${name}`,
        fileName: () => `${name}.js`,
      },
    },
  });
}
for (const f of ['manifest.json', 'popup.html', 'icon.svg']) {
  writeFileSync(join(stage, f), readFileSync(join(here, f)));
}

// --- a stored (uncompressed) zip, enough for an .xpi -----------------------------------------
const table = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = table[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const files = readdirSync(stage).sort();
const locals = [];
const centrals = [];
let offset = 0;
for (const name of files) {
  const data = readFileSync(join(stage, name));
  const nameBuf = Buffer.from(name, 'utf8');
  const crc = crc32(data);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(20, 4); // version needed
  local.writeUInt16LE(0x0800, 6); // UTF-8 names
  local.writeUInt16LE(0, 8); // stored
  local.writeUInt32LE(0x00210000, 10); // a fixed time and date (1980-01-01): reproducible
  local.writeUInt32LE(crc, 14);
  local.writeUInt32LE(data.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(nameBuf.length, 26);
  locals.push(local, nameBuf, data);
  const central = Buffer.alloc(46);
  central.writeUInt32LE(0x02014b50, 0);
  central.writeUInt16LE(20, 4);
  central.writeUInt16LE(20, 6);
  central.writeUInt16LE(0x0800, 8);
  central.writeUInt16LE(0, 10);
  central.writeUInt32LE(0x00210000, 12);
  central.writeUInt32LE(crc, 16);
  central.writeUInt32LE(data.length, 20);
  central.writeUInt32LE(data.length, 24);
  central.writeUInt16LE(nameBuf.length, 28);
  central.writeUInt32LE(offset, 42);
  centrals.push(central, nameBuf);
  offset += 30 + nameBuf.length + data.length;
}
const centralSize = centrals.reduce((n, b) => n + b.length, 0);
const end = Buffer.alloc(22);
end.writeUInt32LE(0x06054b50, 0);
end.writeUInt16LE(files.length, 8);
end.writeUInt16LE(files.length, 10);
end.writeUInt32LE(centralSize, 12);
end.writeUInt32LE(offset, 16);
writeFileSync(join(out, 'formicaria.xpi'), Buffer.concat([...locals, ...centrals, end]));
rmSync(stage, { recursive: true, force: true });
console.log(`addon: dist/addon/formicaria.xpi (${files.join(', ')})`);

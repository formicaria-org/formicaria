// The zip reader formicaria's browser uses for an album bought as a zip (`ui/addon/src/unzip.ts`):
// only the tracks come out, whole, whether the zip stored or deflated them.
import { describe, expect, it } from 'vitest';
import { isAudioName, unzip } from '../../../addon/src/unzip';

const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (b: Uint8Array) => {
  let c = 0xffffffff;
  for (const x of b) c = crcTable[(c ^ x) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};

/** Deflate with the platform's own stream, as the reader inflates with it. */
async function deflateRaw(data: Uint8Array): Promise<Uint8Array> {
  const cs = new CompressionStream('deflate-raw');
  const out = new Response(new Response(data as BodyInit).body!.pipeThrough(cs));
  return new Uint8Array(await out.arrayBuffer());
}

/** A zip of `files`, each stored (0) or deflated (8). */
async function zipOf(
  files: { name: string; data: Uint8Array; method: 0 | 8 }[],
): Promise<Uint8Array> {
  const parts: Uint8Array[] = [];
  const central: Uint8Array[] = [];
  let offset = 0;
  for (const f of files) {
    const name = new TextEncoder().encode(f.name);
    const body = f.method === 8 ? await deflateRaw(f.data) : f.data;
    const local = new DataView(new ArrayBuffer(30));
    local.setUint32(0, 0x04034b50, true);
    local.setUint16(8, f.method, true);
    local.setUint32(14, crc32(f.data), true);
    local.setUint32(18, body.length, true);
    local.setUint32(22, f.data.length, true);
    local.setUint16(26, name.length, true);
    parts.push(new Uint8Array(local.buffer), name, body);
    const c = new DataView(new ArrayBuffer(46));
    c.setUint32(0, 0x02014b50, true);
    c.setUint16(10, f.method, true);
    c.setUint32(16, crc32(f.data), true);
    c.setUint32(20, body.length, true);
    c.setUint32(24, f.data.length, true);
    c.setUint16(28, name.length, true);
    c.setUint32(42, offset, true);
    central.push(new Uint8Array(c.buffer), name);
    offset += 30 + name.length + body.length;
  }
  const size = central.reduce((n, b) => n + b.length, 0);
  const end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true);
  end.setUint16(8, files.length, true);
  end.setUint16(10, files.length, true);
  end.setUint32(12, size, true);
  end.setUint32(16, offset, true);
  const all = [...parts, ...central, new Uint8Array(end.buffer)];
  const out = new Uint8Array(all.reduce((n, b) => n + b.length, 0));
  let at = 0;
  for (const b of all) {
    out.set(b, at);
    at += b.length;
  }
  return out;
}

const bytes = (n: number, seed: number) =>
  Uint8Array.from({ length: n }, (_, i) => (i * seed) & 0xff);

describe('an album bought as a zip', () => {
  it('gives back its tracks, stored or deflated, and nothing else', async () => {
    const one = bytes(5000, 7);
    const two = new Uint8Array(20000).fill(65); // compresses well
    const zip = await zipOf([
      { name: 'Artist - Album/01 Artist - First.flac', data: one, method: 0 },
      { name: 'Artist - Album/02 Artist - Second.mp3', data: two, method: 8 },
      { name: 'Artist - Album/cover.jpg', data: bytes(300, 3), method: 0 },
      { name: 'Artist - Album/', data: new Uint8Array(), method: 0 },
    ]);
    const got = await unzip(zip, isAudioName);
    expect(got.map((e) => e.name)).toEqual(['01 Artist - First.flac', '02 Artist - Second.mp3']);
    expect(got[0].data).toEqual(one);
    expect(got[1].data).toEqual(two);
  });

  it('refuses what is not a zip', async () => {
    await expect(unzip(new Uint8Array(100), isAudioName)).rejects.toThrow('not a zip file');
  });

  it('knows a track by its name', () => {
    expect(isAudioName('a.FLAC')).toBe(true);
    expect(isAudioName('a.m4a')).toBe(true);
    expect(isAudioName('a.pdf')).toBe(false);
  });
});

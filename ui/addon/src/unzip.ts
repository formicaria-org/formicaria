// **Reading a zip in the browser**, for an album bought as a zip (Bandcamp and others): its tracks
// become songs in formicaria, one copy, the zip itself not kept. Only what such archives use: stored
// and deflated entries, no encryption, no zip64. Deflate is the browser's own `DecompressionStream`.

export interface ZipEntry {
  name: string;
  data: Uint8Array;
}

const AUDIO = /\.(mp3|flac|ogg|oga|opus|m4a|aac|wav|aif|aiff|alac|wma)$/i;
export const isAudioName = (n: string) => AUDIO.test(n);

async function inflate(raw: Uint8Array): Promise<Uint8Array> {
  const ds = new DecompressionStream('deflate-raw');
  const out = new Response(new Response(raw as BodyInit).body!.pipeThrough(ds));
  return new Uint8Array(await out.arrayBuffer());
}

/** The entries of `zip` whose names `want` accepts, decompressed. Throws on what it cannot read. */
export async function unzip(zip: Uint8Array, want: (name: string) => boolean): Promise<ZipEntry[]> {
  const dv = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
  // The end-of-central-directory record: within the last 64 KB + 22 bytes.
  let eocd = -1;
  for (let i = zip.length - 22; i >= Math.max(0, zip.length - 65557); i--) {
    if (dv.getUint32(i, true) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) throw new Error('not a zip file');
  const count = dv.getUint16(eocd + 10, true);
  let at = dv.getUint32(eocd + 16, true);
  const dec = new TextDecoder();
  const out: ZipEntry[] = [];
  for (let k = 0; k < count; k++) {
    if (dv.getUint32(at, true) !== 0x02014b50) throw new Error('a damaged zip file');
    const flags = dv.getUint16(at + 8, true);
    const method = dv.getUint16(at + 10, true);
    const size = dv.getUint32(at + 20, true);
    const nameLen = dv.getUint16(at + 28, true);
    const extraLen = dv.getUint16(at + 30, true);
    const commentLen = dv.getUint16(at + 32, true);
    const local = dv.getUint32(at + 42, true);
    const name = dec.decode(zip.subarray(at + 46, at + 46 + nameLen));
    at += 46 + nameLen + extraLen + commentLen;
    if (name.endsWith('/') || !want(name)) continue;
    if (flags & 1) throw new Error('an encrypted zip file');
    if (dv.getUint32(local, true) !== 0x04034b50) throw new Error('a damaged zip file');
    const start = local + 30 + dv.getUint16(local + 26, true) + dv.getUint16(local + 28, true);
    const raw = zip.subarray(start, start + size);
    const data = method === 0 ? raw.slice() : method === 8 ? await inflate(raw) : null;
    if (!data) throw new Error(`a zip entry packed in a way this cannot read (${method})`);
    out.push({ name: name.split('/').pop() ?? name, data });
  }
  return out;
}

// **Adding a photo, on the transport the phone actually uses.**
//
// Until now no test had executed a single line of the Android path. `ipc.ts` chooses its backend
// on `'__TAURI_INTERNALS__' in window`, which is false in jsdom, so `ingestFile`'s base64 branch,
// `shellInvoke`, and the `fmblob://` form of `assetUrl` were unreachable from the suite — and the
// mock's `ingest` arm hashed the *filename* and discarded the bytes, so even a byte-path defect
// that did reach it would have been invisible. That is precisely the blind spot the zero-byte
// photo bug lived in: every capture stored nothing and reported success.
//
// `asPhone()` (see `harness.ts`) installs a fake `__TAURI_INTERNALS__` that models the real shell
// — one `fm` command, a separate `fm_ingest` carrying base64, JSON strings back — and records what
// crosses. So these assert two things nothing could assert before: that the bytes arrive intact,
// and how many of them the bridge has to carry.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { asPhone, asDesktop, fakeFile, type CountingBridge } from './harness';
import * as mock from './mock';

let bridge: CountingBridge;

beforeEach(() => {
  mock.reset();
  bridge = asPhone();
});

afterEach(() => {
  asDesktop();
});

describe('adding a photo on the phone', () => {
  it('carries the bytes intact, and content-addresses them', async () => {
    const { ingestFile } = await import('./ipc');
    const { file, bytes } = fakeFile('DSC_0001.jpg', 300_000);

    const meta = await ingestFile(file, 'personal');

    // The bytes that reached the shell are byte-for-byte the file. Base64 tail handling is the
    // classic place to lose one or two, and losing them would be *silent*: the blob store would
    // hash the truncated payload, mint a perfectly valid-looking reference, and store the wrong
    // photo. (The Rust decoder's own guarantees are pinned in `fm-app`'s `wire` tests.)
    expect(bridge.ingested).toHaveLength(1);
    expect(bridge.ingested[0].length).toBe(bytes.length);
    expect(Array.from(bridge.ingested[0].slice(0, 64))).toEqual(Array.from(bytes.slice(0, 64)));
    expect(Array.from(bridge.ingested[0].slice(-64))).toEqual(Array.from(bytes.slice(-64)));

    // …and the note that comes back names a reference derived from those bytes, not from the
    // filename. Two different files with the same name must not collide.
    expect(meta.assets[0]).toMatch(/^sha256:/);
    const first = meta.assets[0];

    const other = fakeFile('DSC_0001.jpg', 300_001);
    const second = await ingestFile(other.file, 'personal');
    expect(second.assets[0]).not.toBe(first);
  });

  it('carries no more than base64 costs, and only once', async () => {
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('photo.jpg', 240_000);

    await ingestFile(file, 'personal');

    // Base64 is 4 bytes per 3, plus the JSON envelope. Anything materially above this means the
    // payload got copied into the message more than once — which is the failure mode that turns a
    // 12 MP photo into a few hundred megabytes of transient and kills the renderer.
    const carried = bridge.bytes('ingest');
    expect(carried).toBeGreaterThan(240_000); // it really did carry the file
    expect(carried).toBeLessThan(240_000 * 1.4 + 4096);
    expect(bridge.count('ingest')).toBe(1);
  });

  it('refuses a file too large for the bridge, and says why', async () => {
    // The ceiling is a real limitation of the platform, not a policy — Android has no other door
    // for bytes (`InvokeBody::Raw` unsupported; `WebResourceRequest` has no body accessor). What
    // is pinned here is that it is refused *before* anything is encoded, and that the message says
    // what to do instead. An out-of-memory kill mid-encode is the alternative, and it arrives with
    // no message at all.
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('holiday.mp4', 20 * 1024 * 1024, 'video/mp4');

    await expect(ingestFile(file, 'personal')).rejects.toThrow(/ceiling|MB/);
    // Nothing was encoded or sent.
    expect(bridge.count('ingest')).toBe(0);
    expect(bridge.ingested).toHaveLength(0);
  });

  it('keeps a phone photo comfortably under the ceiling', async () => {
    // A 48 MP JPEG is ~12 MB, which is the case the ceiling exists to serve. If someone lowers it
    // further to chase memory, this is what should stop them.
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('48mp.jpg', 12 * 1024 * 1024);
    await expect(ingestFile(file, 'personal')).resolves.toBeTruthy();
  });
});

describe('reading a blob back on the phone', () => {
  it('points at the shell protocol handler, not the desktop HTTP route', async () => {
    // The trap this pins, in `ipc.ts`'s own words: "PROD is not a statement about which backend is
    // present." A Tauri build is `import.meta.env.PROD`, so the phone once claimed to stream and
    // then pointed every image at `/api/blob/…` — a route that exists only in `fm-serve`. Every
    // image on the device failed. Now that a test can *be* the phone, the two branches are
    // checkable rather than reasoned about.
    const { assetUrl, streamsBlobs } = await import('./ipc');
    expect(streamsBlobs()).toBe(true);
    expect(assetUrl('sha256:abc')).toContain('fmblob');
    expect(assetUrl('sha256:abc')).not.toContain('/api/blob/');

    asDesktop();
    expect(assetUrl('sha256:abc')).toBe('/api/blob/sha256%3Aabc');
  });
});

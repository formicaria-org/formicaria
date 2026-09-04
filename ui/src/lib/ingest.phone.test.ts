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

  /// **This test used to assert the opposite, and the reversal is the point.**
  ///
  /// It read *"refuses a file too large for the bridge, and says why"*, and it was right while
  /// the single-shot message was the only door: a 20 MB video encoded into one JSON argument is
  /// an out-of-memory kill, and a refusal with a sentence beats a kill with none.
  ///
  /// Chunked ingest (2026-09-04) removed the reason. The ceiling was never about attachment size
  /// — it was the point at which base64-in-JSON stopped fitting — so with the file sliced there
  /// is nothing left to refuse. What survives is the *shape* of the old assertion: nothing over
  /// the ceiling may go through the single-shot path, because that path is still the one that
  /// cannot carry it. See `a large file on the phone` below for where it goes instead.
  it('never sends an oversized file through the single-shot path', async () => {
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('holiday.mp4', 20 * 1024 * 1024, 'video/mp4');

    await expect(ingestFile(file, 'personal')).resolves.toBeTruthy();
    expect(bridge.count('ingest')).toBe(0);
  });

  it('keeps a phone photo on the cheap single-shot path', async () => {
    // A 48 MP JPEG is ~12 MB, which is the case the ceiling exists to serve. Now that anything
    // larger is sliced rather than refused, what this pins is that the *common* case still takes
    // one message: if someone lowers `MAX_INGEST` to chase memory, every phone photo silently
    // becomes five round trips, and this is what should stop them.
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('48mp.jpg', 12 * 1024 * 1024);
    await expect(ingestFile(file, 'personal')).resolves.toBeTruthy();
    expect(bridge.count('ingest')).toBe(1);
    expect(bridge.count('ingest_chunk')).toBe(0);
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

// **A file over the single-shot ceiling is sliced, not refused** (2026-09-04).
//
// `MAX_INGEST` was never a judgement about how large an attachment ought to be: it is the point
// at which base64 in a JSON argument, copied several times between here and Rust, stops fitting
// on a phone — `outstanding.md` §1.3, *"a memory limit wearing a size limit's clothes"*. It is
// what refused video. These assert the three things that make chunking safe to prefer.
describe('a large file on the phone', () => {
  it('is sliced instead of refused, and arrives in order', async () => {
    const { ingestFile } = await import('./ipc');
    // Over the 16 MB single-shot ceiling. The old code threw here.
    const { file, bytes } = fakeFile('holiday.mp4', 20 * 1024 * 1024, 'video/mp4');

    const meta = await ingestFile(file, 'personal');

    // It went chunked, and it went in more than one piece — a "chunked" path that sent the whole
    // file as chunk 0 would pass every other assertion here while fixing nothing.
    expect(bridge.count('ingest_chunk')).toBeGreaterThan(1);
    expect(bridge.count('ingest')).toBe(0);
    expect(bridge.count('ingest_finish')).toBe(1);

    // **Assembled in the right order, byte for byte.** Out-of-order slices still produce a
    // plausible file with a valid-looking reference; only the content gives it away.
    expect(bridge.ingested).toHaveLength(1);
    expect(bridge.ingested[0].length).toBe(bytes.length);
    expect(Array.from(bridge.ingested[0].slice(0, 64))).toEqual(Array.from(bytes.slice(0, 64)));
    const mid = Math.floor(bytes.length / 2);
    expect(Array.from(bridge.ingested[0].slice(mid, mid + 64))).toEqual(
      Array.from(bytes.slice(mid, mid + 64)),
    );
    expect(Array.from(bridge.ingested[0].slice(-64))).toEqual(Array.from(bytes.slice(-64)));

    expect(meta.assets[0]).toMatch(/^sha256:/);
  });

  // No slice may be larger than what the backend accepts (`fm_core::chunked::MAX_CHUNK`, 4 MB).
  // That limit exists precisely so a frontend cannot reintroduce the problem it solves.
  it('keeps every slice under the backend chunk limit', async () => {
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('holiday.mp4', 20 * 1024 * 1024, 'video/mp4');
    await ingestFile(file, 'personal');

    const MAX_CHUNK = 4 * 1024 * 1024;
    for (const call of bridge.calls.filter((c) => c.cmd === 'ingest_chunk')) {
      expect(call.bytes).toBeLessThan(MAX_CHUNK * 1.4); // base64 is ~4/3, plus the JSON wrapper
    }
  });

  // A small file must keep taking the single-shot path: one message is cheaper than five, and the
  // path every photo already uses should not be rerouted through new code for no gain.
  it('leaves the ordinary photo path alone', async () => {
    const { ingestFile } = await import('./ipc');
    const { file } = fakeFile('DSC_0002.jpg', 300_000);
    await ingestFile(file, 'personal');

    expect(bridge.count('ingest')).toBe(1);
    expect(bridge.count('ingest_chunk')).toBe(0);
  });
});

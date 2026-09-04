/// Put the test on a phone.
///
/// **Why this file exists.** The suite had 39 UI test files and not one of them had ever run a
/// line of the Android code path. Two things stood in the way, and both are removed here:
///
///  - `ipc.ts` picks its backend on `'__TAURI_INTERNALS__' in window`. In jsdom that is false, so
///    every test took the in-memory mock and the `shellInvoke` / `fm_ingest` / `fmblob://` branches
///    — the ones the phone actually runs — were dead code.
///  - jsdom ships no `matchMedia`, so every `(pointer: coarse)` branch was dead too. `test-setup.ts`
///    supplies the stub; this flips it.
///
/// **What this models, and what it cannot.** It models the *shape* of the Android bridge: one
/// `fm` command carrying `(cmd, args)`, a separate `fm_ingest` carrying the file base64 inside a
/// JSON string, and every reply coming back as a JSON **string** rather than an object — which is
/// a real source of bugs (`shellInvoke` parses centrally precisely because forgetting to hands
/// back a `string` that type-checks as whatever was asked for).
///
/// It does **not** model Android's synchronous JNI bridge, where `window.ipc.postMessage` parks
/// the JS thread until Rust returns. Nothing in jsdom can. That is why the budgets these helpers
/// support count **work** — calls made, bytes carried — rather than wall-clock: on a phone the
/// cost of a command is the cost of blocking the UI for it, so counting commands measures the
/// thing that hurts, deterministically and on any machine.
import * as mock from './mock';
import { setCoarsePointer } from '../test-setup';

/// One command as it crossed the bridge.
export interface BridgeCall {
  /// The dispatch name, unwrapped from the `fm` envelope — `get`, `backlinks`, `ingest`, …
  /// so a budget reads in the app's own vocabulary rather than counting `fm` 400 times.
  cmd: string;
  /// How many bytes the arguments occupied as serialised JSON. This is the number that kills a
  /// phone: it is copied roughly ten times between the page and Rust, so a budget on it is a
  /// budget on peak memory.
  bytes: number;
}

/// A recording bridge. Every `invoke` is logged, then served by the in-memory mock, so the app
/// under test behaves normally while the test gets an exact ledger of what it asked for.
export interface CountingBridge {
  calls: BridgeCall[];
  /// How many times a given dispatch name crossed the bridge.
  count(cmd: string): number;
  /// Total argument bytes for a given dispatch name (all of them, if omitted).
  bytes(cmd?: string): number;
  /// Bytes handed to `fm_ingest`, decoded back to the file they encode — so a test can assert
  /// the photo arrived intact, which no test could observe before. A chunked upload contributes
  /// one entry too, assembled from its slices at `fm_ingest_finish`.
  ingested: Uint8Array[];
  reset(): void;
}

function decodeBase64(data: string): Uint8Array {
  const bin = atob(data);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) out[i] = bin.charCodeAt(i);
  return out;
}

/// Install a fake `__TAURI_INTERNALS__` and switch the pointer to coarse.
///
/// Returns the ledger. Call `asDesktop()` in an `afterEach` — the global outlives the test
/// otherwise, and vitest isolates per *file*, not per test.
export function asPhone(): CountingBridge {
  const calls: BridgeCall[] = [];
  const ingested: Uint8Array[] = [];

  const bridge: CountingBridge = {
    calls,
    ingested,
    count: (cmd) => calls.filter((c) => c.cmd === cmd).length,
    bytes: (cmd) =>
      calls.filter((c) => cmd === undefined || c.cmd === cmd).reduce((n, c) => n + c.bytes, 0),
    reset: () => {
      calls.length = 0;
      ingested.length = 0;
    },
  };

  /// **The chunked upload path** (`fm_ingest_chunk` / `fm_ingest_finish` / `fm_ingest_cancel`),
  /// modelled the way `fm_core::chunked` behaves rather than the way it is convenient to fake:
  /// slices are appended in order and an out-of-order `seq` is **refused**, because a harness that
  /// accepted one would let a frontend bug through that the device would reject.
  const sessions = new Map<string, { parts: number[][]; next: number }>();

  /// The shell's commands, in the shapes `mobile/src-tauri/src/lib.rs` really exposes.
  async function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<string> {
    if (cmd === 'fm_ingest_chunk') {
      const { session, seq, data } = args as { session: string; seq: number; data: string };
      const s = sessions.get(session) ?? { parts: [], next: 0 };
      if (seq !== s.next) {
        throw new Error(`upload '${session}' expected chunk ${s.next} and got ${seq}`);
      }
      s.parts.push(Array.from(decodeBase64(data)));
      s.next += 1;
      sessions.set(session, s);
      calls.push({ cmd: 'ingest_chunk', bytes: JSON.stringify(args).length });
      return JSON.stringify({ session, received: s.parts.flat().length });
    }
    if (cmd === 'fm_ingest_finish') {
      const { session, name, vault } = args as { session: string; name: string; vault: string };
      const s = sessions.get(session);
      if (!s) throw new Error(`upload '${session}' has no bytes`);
      sessions.delete(session);
      // Assembled in order — which is the whole thing under test on this side: the file the
      // vault ends up with must be the file that was chosen, in the right order.
      const bytes = new Uint8Array(s.parts.flat());
      ingested.push(bytes);
      calls.push({ cmd: 'ingest_finish', bytes: JSON.stringify(args).length });
      return JSON.stringify(await mock.handle('ingest', { name, vault, bytes }));
    }
    if (cmd === 'fm_ingest_cancel') {
      const { session } = args as { session: string };
      sessions.delete(session);
      calls.push({ cmd: 'ingest_cancel', bytes: JSON.stringify(args).length });
      return '';
    }
    if (cmd === 'fm_ingest') {
      const { name, vault, data } = args as { name: string; vault: string; data: string };
      // The one place a test can see the bytes. `lib.rs` base64-decodes and hands them to
      // `dispatch("ingest", …)`; the mock's `ingest` arm now hashes what it is given, so a
      // truncated or dropped payload changes the reference and the assertion fails — which is
      // exactly the defect ("every photo stored as zero bytes") that shipped undetected.
      const bytes = decodeBase64(data);
      ingested.push(bytes);
      calls.push({ cmd: 'ingest', bytes: JSON.stringify(args).length });
      return JSON.stringify(await mock.handle('ingest', { name, vault, bytes }));
    }
    if (cmd === 'fm') {
      const inner = args as { cmd: string; args: Record<string, unknown> };
      calls.push({ cmd: inner.cmd, bytes: JSON.stringify(inner.args ?? {}).length });
      const out = await mock.handle(inner.cmd, inner.args ?? {});
      // The shell answers with a JSON string, and `undefined` for a void command — matching
      // `String::from_utf8(out.into_bytes())` and `shellInvoke`'s `text ? JSON.parse(text) : …`.
      return out === undefined ? '' : JSON.stringify(out);
    }
    // `agent_activity_poll` and friends are shell-answered arms; anything genuinely unknown
    // should fail the way the device fails, loudly.
    calls.push({ cmd, bytes: JSON.stringify(args).length });
    throw new Error(`unknown command: ${cmd}`);
  }

  (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
    invoke,
    // `blobBase` probes this to learn where the `fmblob` handler answers, mirroring wry's
    // Android rewrite of `fmblob://…` to `http://fmblob.localhost/…`.
    convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
    transformCallback: (cb: unknown) => cb,
  };
  setCoarsePointer(true);
  return bridge;
}

/// Undo `asPhone()`. Safe to call unconditionally.
export function asDesktop(): void {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  setCoarsePointer(false);
}

/// A `File` of `size` bytes with deterministic, non-repeating content.
///
/// Not `'x'.repeat(n)`: identical bytes hide an off-by-one in a base64 round trip, and the blob
/// store is content-addressed, so a test that cannot tell two payloads apart cannot tell whether
/// the right one arrived. jsdom's `File` has no `arrayBuffer()` worth trusting, so the bytes are
/// kept alongside for the assertion.
export function fakeFile(name: string, size: number, type = 'image/jpeg'): {
  file: File;
  bytes: Uint8Array;
} {
  const bytes = new Uint8Array(size);
  for (let i = 0; i < size; i += 1) bytes[i] = (i * 31 + (i >> 8)) & 0xff;
  return { file: new File([bytes], name, { type }), bytes };
}

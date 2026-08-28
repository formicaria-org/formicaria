/// Which machine is this bundle running on?
///
/// One question, asked in one place, because the same bundle ships to three backends and the
/// answer decides which one it talks to. It used to be a module-scope `const` in `ipc.ts`:
///
/// ```ts
/// const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
/// ```
///
/// That is correct in production — Tauri puts `__TAURI_INTERNALS__` on `window` before any of our
/// code runs — and it is **untestable**, which is why the phone's transport had never once been
/// executed by a test. A `const` is evaluated at import time, so a test can only reach the phone
/// branch by resetting the module registry and re-importing every module that transitively pulls
/// `ipc.ts` in, which for `App.svelte` is all of them. A function is read at call time, so
/// `asPhone()` in `harness.ts` flips it for the whole app with one assignment.
///
/// The lazy read costs one property lookup per command and is strictly safer besides: nothing now
/// depends on this module being imported after Tauri's injection.

/// Is the app running inside the Tauri shell — i.e. on the phone?
///
/// **Tauri means Android here, and only Android.** The desktop is a browser pointed at `fm-serve`
/// (the native window was removed — see `decisions.md`), so there is no desktop Tauri build to
/// confuse this with. That is what lets callers read this as "am I on a battery device with no
/// HTTP server", which is the question `checkRemotes` and `ingestFile` are really asking.
export function isPhone(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/// Is the pointer a finger?
///
/// Centralised for the same reason as `isPhone`: jsdom ships no `matchMedia` at all, so every
/// `(pointer: coarse)` branch in the app was dead code in CI until `test-setup.ts` supplied a stub
/// this can read. Guarded rather than assumed, because a missing `matchMedia` should mean "not a
/// touch device", never a `TypeError` at module scope.
export function coarsePointer(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(pointer: coarse)').matches;
}

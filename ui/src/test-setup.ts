/// Suite-wide test configuration.
///
/// **Why this exists: a 1-second default that is not about the code.** Testing Library's `findBy*`
/// helpers retry for `asyncUtilTimeout`, 1000 ms by default. Most files here mount the whole `App` —
/// jsdom, Svelte hydration, lazy panel imports — and with the files running in parallel, one extra
/// test file was enough to push a heavy mount past that second. The symptom was a test failing in
/// `App.features.test.ts` while the file that "caused" it touched nothing it uses (2026-07-31), which
/// sends you hunting for shared state that vitest already isolates per file.
///
/// So the timeout is set from what a loaded machine actually needs, not from a library default. It
/// bounds only *waiting*: a `findBy*` that will succeed is unaffected, and a genuine failure costs a
/// few extra seconds once. Deliberately not a `testTimeout` bump — that would hide a hang.
import { configure } from '@testing-library/svelte';

configure({ asyncUtilTimeout: 5000 });

/// **jsdom ships no `matchMedia`, and that silently deleted the phone from the test suite.**
///
/// Every `(pointer: coarse)` branch in the app — the persistent format bar, the tap→move menu
/// path, the touch shells — is chosen by `window.matchMedia('(pointer: coarse)').matches`. With no
/// `matchMedia` at all, `platform.ts`'s guarded read answers `false` and those branches were
/// unreachable in CI for the entire life of the project. `App.svelte` says as much in a comment:
/// *"Verified at a narrow viewport and by `pointer: coarse`, NOT on a device."*
///
/// So the suite supplies one. It **defaults to a fine pointer**, which is what every existing test
/// already assumed, so nothing changes for them; `asPhone()` in `lib/harness.ts` flips it.
///
/// Deliberately not a `vi.fn()`: `clearMocks: true` in `vitest.config.ts` resets mock
/// implementations between tests, which would strip the stub out from under any test that set it
/// up in a `beforeAll`. A plain function survives.
let coarse = false;

/// Set by `asPhone()` / `asDesktop()`. Exported rather than reached through `window` so a test
/// that wants a finger without the rest of the phone can say exactly that.
export function setCoarsePointer(on: boolean): void {
  coarse = on;
}

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  configurable: true,
  value: (query: string): MediaQueryList =>
    ({
      // The app asks two things of it. `(pointer: coarse)` is the one that picks touch branches;
      // anything else (a width query, `prefers-reduced-motion`) answers false, which is jsdom's
      // own posture — it applies no CSS, so there is no viewport to report on honestly.
      matches: query.includes('pointer: coarse') ? coarse : false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList,
});

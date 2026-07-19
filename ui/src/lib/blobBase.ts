// Where the shell's blob protocol answers — derived, never written down.
//
// **Its own module, with no imports.** It was a function inside `ipc.ts`, and a test for it had
// to import `ipc`, which imports the in-memory `mock` — whose module-level counter dates the
// fixture notes. That was enough to shift the dates another test grouped by, and a passing suite
// went red from a file that only added assertions. A pure function with no dependencies cannot
// do that to anything.

/** The origin the shell's `fmblob` handler answers on, with a trailing slash.
 *
 *  Derived from `convertFileSrc` rather than written out, because a custom scheme is not the
 *  same string on every platform: Android's WebView cannot intercept one at all, so wry rewrites
 *  `fmblob://…` to `http://fmblob.localhost/…` (its `custom_protocol_workaround`). Asking Tauri
 *  to map a known path and then trimming it back is how this stays correct on a platform whose
 *  rewriting rules are not ours. */
export function blobBase(
  convert?: (path: string, protocol: string) => string,
): string {
  const FALLBACK = 'fmblob://localhost/';
  // **Never throws.** This is reached from `assetUrl`, which runs *during render* of any note
  // that has an asset — so an exception here does not surface as a failed image, it takes the
  // whole component down and the app is a blank screen with nothing in the console. That is
  // exactly what shipped: the emulator's vault is empty so `assetUrl` never ran there, while a
  // real vault with one asset note blanked on launch.
  //
  // A wrong URL costs a broken image. A throw costs the application.
  try {
    if (!convert) return FALLBACK;
    const probe = convert('__base__', 'fmblob');
    const cut = probe.lastIndexOf('__base__');
    return cut > 0 ? probe.slice(0, cut) : FALLBACK;
  } catch {
    return FALLBACK;
  }
}

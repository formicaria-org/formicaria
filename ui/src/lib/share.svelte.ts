// Pictures shared to formicaria from another app — on Android, the **Share** button under a
// screenshot (`decisions.md#ui`, 2026-10-20, *a picture shared from another app goes into a note*).
//
// The shell copies what was shared into the app's own storage and holds it (`__fmShare`, written
// by `ci/android-inject-service.sh`); the page asks for it when it starts and whenever the shell
// says more has arrived (`fm-share`). The same "the page asks; the shell does not tell" shape as
// the insets, because at a cold start the page does not exist yet when Android hands it over.
//
// On a computer there is no bridge, nothing is ever pending, and none of this is seen.

type Bridge = {
  pending(): string;
  read(i: number): string;
  done(files: number, refused: number): void;
};
const bridge = (): Bridge | null => (globalThis as { __fmShare?: Bridge }).__fmShare ?? null;

/** What could not be taken, and why: `too large` or `could not be read`. */
export interface Refused {
  name: string;
  why: string;
}

/** What the shell holds: the files it kept, and any it could not take. */
interface Pending {
  files: { name: string; mime: string }[];
  refused: Refused[];
}

/** Take what was shared, as files, and let the shell forget it. Empty when nothing was. */
export function takeShared(): { files: File[]; refused: Refused[] } {
  const b = bridge();
  if (!b) return { files: [], refused: [] };
  let pending: Pending;
  try {
    pending = JSON.parse(b.pending());
  } catch {
    return { files: [], refused: [] };
  }
  if (!pending.files.length && !pending.refused?.length) return { files: [], refused: [] };
  const files = pending.files.map((f, i) => {
    const bin = atob(b.read(i));
    const bytes = new Uint8Array(bin.length);
    for (let k = 0; k < bin.length; k++) bytes[k] = bin.charCodeAt(k);
    return new File([bytes], f.name, { type: f.mime });
  });
  // By count: a second share still being copied in the shell is not thrown away with this one.
  b.done(pending.files.length, pending.refused?.length ?? 0);
  return { files, refused: pending.refused ?? [] };
}

/** Files on their way into a note. The note panel showing `noteId` takes them, at the end of the
 *  note, through the same path as a dropped file, and clears this. */
export const incoming = $state<{ value: { noteId: string; files: File[] } | null }>({
  value: null,
});

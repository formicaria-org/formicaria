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
  /** Texts taken (by count). Absent in a shell from before 0.39. */
  doneTexts?(n: number): void;
};
const bridge = (): Bridge | null => (globalThis as { __fmShare?: Bridge }).__fmShare ?? null;

/** What could not be taken, and why: `too large` or `could not be read`. */
export interface Refused {
  name: string;
  why: string;
}

/** Text brought in from another app: Share's text (a link, a passage), or a selection whose menu
 *  offered formicaria (`decisions.md` 2026-11-16, *the phone's Bring in*). */
export interface SharedText {
  text: string;
  subject: string;
}

/** What the shell holds: the files it kept, any it could not take, and texts. */
interface Pending {
  files: { name: string; mime: string }[];
  refused: Refused[];
  texts?: SharedText[];
}

/** Take what was shared, as files and texts, and let the shell forget it. Empty when nothing was. */
export function takeShared(): { files: File[]; refused: Refused[]; texts: SharedText[] } {
  const none = { files: [], refused: [], texts: [] };
  const b = bridge();
  if (!b) return none;
  let pending: Pending;
  try {
    pending = JSON.parse(b.pending());
  } catch {
    return none;
  }
  const texts = (pending.texts ?? []).filter((t) => typeof t?.text === 'string' && t.text.trim());
  if (pending.texts?.length) b.doneTexts?.(pending.texts.length);
  if (!pending.files.length && !pending.refused?.length) return { files: [], refused: [], texts };
  const files = pending.files.map((f, i) => {
    const bin = atob(b.read(i));
    const bytes = new Uint8Array(bin.length);
    for (let k = 0; k < bin.length; k++) bytes[k] = bin.charCodeAt(k);
    return new File([bytes], f.name, { type: f.mime });
  });
  // By count: a second share still being copied in the shell is not thrown away with this one.
  b.done(pending.files.length, pending.refused?.length ?? 0);
  return { files, refused: pending.refused ?? [], texts };
}

/** Files on their way into a note. The note panel showing `noteId` takes them, at the end of the
 *  note, through the same path as a dropped file, and clears this. */
export const incoming = $state<{ value: { noteId: string; files: File[] } | null }>({
  value: null,
});

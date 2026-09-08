import { getNote } from './ipc';

/** The note id inside a conflict file path (`notes/<ULID>.md`), or the raw path if it doesn't match. */
export function conflictId(path: string): string {
  const m = /([0-9A-HJKMNP-TV-Za-z]{26})\.md$/.exec(path);
  return m ? m[1] : path;
}

/** A line git wrote to mark a conflict: seven or more of `<`, `=`, `>` or `|`, then the end of the
 *  line or a space before its label. **All four, not the two with labels on them** — an `=======`
 *  is what a hunk with an empty *ours* side begins with, and a note named `“=======”` in three
 *  separate notices was the result. Seven is git's default and the only size this app asks for;
 *  the `+` tolerates a repo that configured a larger one. */
const MARKER = /^(<{7,}|={7,}|>{7,}|\|{7,})(\s|$)/;

/** Turn conflict file paths into human labels so the message names the **note**, not a ULID: the
 *  title if it has one, else its first line, else the bare id. A conflicted note still opens (the
 *  markers are in the body), so `get` resolves it; a lookup that fails degrades to the id. */
export async function conflictLabels(paths: string[]): Promise<string[]> {
  return Promise.all(
    paths.map(async (p) => {
      const id = conflictId(p);
      const note = await getNote(id).catch(() => null);
      const title = note?.title?.trim();
      if (title) return `“${title}”`;
      const firstLine = note?.body
        ?.split('\n')
        .map((l) => l.replace(/^#+\s*/, '').trim())
        .find((l) => l && !MARKER.test(l));
      return firstLine ? `“${firstLine.slice(0, 48)}”` : id;
    }),
  );
}

import { getNote } from './ipc';

/** The note id inside a conflict file path (`notes/<ULID>.md`), or the raw path if it doesn't match. */
export function conflictId(path: string): string {
  const m = /([0-9A-HJKMNP-TV-Za-z]{26})\.md$/.exec(path);
  return m ? m[1] : path;
}

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
        .find((l) => l && !l.startsWith('<<<<<<<') && !l.startsWith('>>>>>>>'));
      return firstLine ? `“${firstLine.slice(0, 48)}”` : id;
    }),
  );
}

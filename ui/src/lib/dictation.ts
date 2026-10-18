// **Dictation, once** (`decisions.md` 2026-10-18): what every place that listens shares — the note's
// editor and Food's *Dictate a recipe*. A recording goes to this device's speech model and comes
// back as the words that were said; when it cannot, the caller is told why and keeps the recording
// (nothing said is ever lost). The recorder itself is `record.ts`.
import { dictate } from './ipc';

/** A dictation stops itself here: the phone carries it in one message, and five minutes of speech
 *  is ~10 MB there, well inside what that message holds (`MAX_INGEST`). */
export const DICTATION_SECS = 300;

/** The words a recording holds, or why there are none — never both empty. */
export async function hear(file: File): Promise<{ words: string; why: string }> {
  try {
    const words = await dictate(file);
    return words
      ? { words, why: '' }
      : { words: '', why: 'Nothing could be written down from that recording.' };
  } catch (e) {
    return { words: '', why: e instanceof Error ? e.message : String(e) };
  }
}

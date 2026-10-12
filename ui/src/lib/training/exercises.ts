/// The carried exercise cards, asked for once: the file is the same until the app is rebuilt, and
/// two screens want it (the cards themselves, and a session that links an exercise to its card).
import { exerciseLibrary } from '../ipc';
import type { ExerciseLibrary } from '../types';

let asked: Promise<ExerciseLibrary> | null = null;
export function exercises(): Promise<ExerciseLibrary> {
  asked ??= exerciseLibrary().catch((e) => {
    asked = null; // a failure is not remembered
    throw e;
  });
  return asked;
}
/** For tests: forget what was fetched. */
export const forget = () => void (asked = null);

/** A name as two spellings of one exercise share it: case, punctuation and a plural aside —
 *  "Squat" typed in a session is the card called "Squats". */
const plain = (name: string) =>
  name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .split(' ')
    .map((w) => (w.length > 3 && w.endsWith('s') && !w.endsWith('ss') ? w.slice(0, -1) : w))
    .join(' ')
    .trim();
export const sameName = (a: string, b: string) => plain(a) === plain(b);

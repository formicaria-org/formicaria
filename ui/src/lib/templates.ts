// What a template hands to the note made from it, and the templates a vault can be started with
// (`decisions.md#ui`, 2026-10-12, *a template carries structure*).
//
// A template is a note tagged `template`. The note made from it gets its body, its tags but for
// that one, its status and its own properties — so it arrives with the structure a view reads.
// What says *which* note something is, or *when*, never travels.
import type { NoteDetail, NoteDraft } from './types';

export const TEMPLATE_TAG = 'template';
/** A property the first starter templates carried, to say which group offered them. Nothing reads
 *  it since the plus became one menu again (2026-10-12); it is still never copied. */
export const GROUP = 'group';

const NEVER = new Set([
  'id',
  'created',
  'updated',
  'schema',
  'title',
  'start',
  'due',
  // A logged entry's key hides whatever carries it, and is set only where an entry is made.
  'log',
  'thread_of',
  'reply_to',
  'proposes',
  GROUP,
]);

/** The tag and the key of a recipe's own note (`decisions.md#ui`, 2026-10-12). */
export const RECIPE = 'recipe';
/** **One shape for a recipe's own note, however it is started** — from the Recipe template or from
 *  the food view (owner, 2026-10-18: *"they should be the same"*). Only the ingredients are counted,
 *  and they are kept apart; everything else a recipe is — its ingredients as written, method, times,
 *  notes, pictures — is this text, free. Only the Ingredients list is read back, as lines to match
 *  to foods (`food/recipeText.ts`). */
export const RECIPE_PARTS = {
  ingredients: 'Ingredients',
  method: 'Method',
  times: 'Times',
  notes: 'Notes',
} as const;
const P = RECIPE_PARTS;
export const RECIPE_BODY = `## ${P.ingredients}\n\n- \n\n## ${P.method}\n\n1. \n\n## ${P.times}\n\n\n## ${P.notes}\n\n`;
/** A fresh word for a recipe: what its note and its ingredients share, and nothing else does. */
export const recipeWord = () =>
  globalThis.crypto?.randomUUID?.().slice(0, 13) ??
  `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;

/** The note a template makes: its structure, and nothing that was the template's own. */
export function draftFrom(t: NoteDetail): NoteDraft {
  const props: Record<string, string | number> = {};
  for (const [k, v] of Object.entries(t.props ?? {})) {
    if (NEVER.has(k) || k.startsWith('conflict-')) continue;
    if (typeof v === 'string' || (typeof v === 'number' && Number.isInteger(v))) props[k] = v;
  }
  if (t.status) props.status = t.status;
  // **A recipe gets a word of its own, never the template's.** The word is what joins a recipe to
  // its ingredients, so two recipes sharing one would share their ingredients too.
  if (t.tags.includes(RECIPE)) props[RECIPE] = recipeWord();
  return {
    title: '',
    body: t.body ?? '',
    tags: t.tags.filter((x) => x !== TEMPLATE_TAG),
    props,
  };
}

/// **Starter templates are real notes**, written into a vault when a person asks — none is held
/// in the app, because a template that is not a file would not be a note.
///
/// **Each has a body**: a heading and its sections. A note made from one whose structure was all
/// in its tags and properties opened as a blank page (`decisions.md#ui`, 2026-10-12, the owner on
/// his phone: "looks like empty notes?"). **No target and no advice** (`decisions.md#data`,
/// 2026-10-11, *nothing is invented*) — and no template for a program, whose targets are set
/// from the day screen's own form.
export function starters(firstStatus: string): NoteDraft[] {
  const made = (title: string, tag: string, body: string, props = {}) => ({
    title,
    body,
    tags: [TEMPLATE_TAG, tag],
    props,
  });
  return [
    made(
      'Task',
      'task',
      '# New task\n\n## What\n\n\n## Steps\n\n- [ ] \n',
      firstStatus ? { status: firstStatus } : {},
    ),
    made('Meeting', 'meeting', '# Meeting\n\n## Who\n\n\n## Notes\n\n\n## Actions\n\n- [ ] \n'),
    made('Recipe', RECIPE, `# New recipe\n\n${RECIPE_BODY}`, { servings: 1 }),
  ];
}

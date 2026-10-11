// What a template hands to the note made from it, and the templates a vault can be started with
// (`decisions.md#ui`, 2026-10-12, *a template carries structure*).
//
// A template is a note tagged `template`. The note made from it gets its body, its tags but for
// that one, its status and its own properties — so it arrives with the structure a view reads.
// What says *which* note something is, or *when*, never travels.
import type { NoteDetail, NoteDraft, ObjectMeta } from './types';

export const TEMPLATE_TAG = 'template';
/** The property by which a template says which group offers it. Never copied. */
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

/** The templates a group offers: the ones that name it. */
export const templatesOf = (all: ObjectMeta[], group: string) =>
  all.filter((t) => t.props?.[GROUP] === group);

/// **Starter templates are real notes**, written into a vault when a person asks — none is held
/// in the app, because a template that is not a file would not be a note. Each carries only
/// structure: a tag, the group that offers it, the property a view reads. **No number, no target
/// and no advice** (`decisions.md#data`, 2026-10-11, *nothing is invented*).
export function starters(firstStatus: string): NoteDraft[] {
  const made = (title: string, tag: string, group: string, body: string, props = {}) => ({
    title,
    body,
    tags: [TEMPLATE_TAG, tag],
    props: { [GROUP]: group, ...props },
  });
  return [
    made('Task', 'task', 'Tasks', '', firstStatus ? { status: firstStatus } : {}),
    made('Meeting', 'meeting', 'Library', '## Who\n\n## Notes\n\n## Actions\n\n- [ ] \n'),
    made('Recipe', RECIPE, 'Food', '## Method\n\n', { servings: 1 }),
    made('Diet program', 'program', 'Food', '', { program: 'food' }),
    made('Training program', 'program', 'Fitness', '', { program: 'training' }),
  ];
}

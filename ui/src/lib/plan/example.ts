/// A regime's example week, turned into routines a person then owns and changes
/// (`decisions.md#data`, 2026-10-12, *every regime has a week to start from*).
///
/// Each different thing in the week becomes one routine, meant for the weekdays it falls on: a
/// session is its exercises as sets, an activity is itself. Nothing is planned onto a day here —
/// the week screen offers each routine on its days, as for any routine.
import type { ExerciseCard, NoteDraft, PlanExample, PlanExampleItem } from '../types';

const REGIONS = ['upper-push', 'upper-pull', 'lower-push', 'lower-pull'];
/** The first whole number in a source's "8 to 12" or "10 steps each way": where to start. */
export const firstNumber = (text: string | number | undefined): number => {
  const m = /\d+/.exec(String(text ?? ''));
  return m ? Number(m[0]) : 0;
};

/** One line for an item, as a person reads it in the week. */
export function itemLine(item: PlanExampleItem, cards: ExerciseCard[]): string {
  if (item.type === 'activity')
    return `${item.name}${item.minutes ? ` · ${item.minutes} min` : ''}`;
  const names = (item.exercises ?? []).map((x) => {
    const card = cards.find((c) => c.id === x.card);
    return `${card?.name ?? x.card} ${x.sets} × ${x.reps}`;
  });
  return `${item.name}: ${names.join('; ')}`;
}

export function routinesFrom(
  example: PlanExample,
  cards: ExerciseCard[],
  stamp: string,
): NoteDraft[] {
  const seen = new Map<string, { item: PlanExampleItem; days: string[] }>();
  for (const d of example.week)
    for (const item of d.items) {
      const { said: _said, ...rest } = item as PlanExampleItem & { said?: string };
      const key = JSON.stringify(rest);
      const had = seen.get(key);
      if (had) had.days.push(d.day);
      else seen.set(key, { item, days: [d.day] });
    }
  const drafts: NoteDraft[] = [];
  let n = 0;
  for (const { item, days } of seen.values()) {
    const word = `routine-${stamp}-${n++}`;
    drafts.push({
      title: item.name,
      start: '',
      props: { routine: word, weekdays: days.join(',') },
    });
    if (item.type === 'activity') {
      drafts.push({
        title: item.name,
        start: '',
        props: {
          log: 'workout',
          routine: word,
          activity: item.name,
          counts_as: item.counts_as ?? 'aerobic',
          minutes: item.minutes ?? 0,
          effort: item.effort ?? 0,
        },
      });
      continue;
    }
    for (const x of item.exercises ?? []) {
      const card = cards.find((c) => c.id === x.card);
      const name = card?.name ?? x.card;
      for (let i = 1; i <= Math.max(1, x.sets); i++)
        drafts.push({
          title: name,
          start: '',
          props: {
            log: 'workout',
            routine: word,
            exercise: name,
            set: i,
            reps: firstNumber(x.reps),
            load_g: 0,
            ...(card && REGIONS.includes(card.area) ? { region: card.area } : {}),
          },
        });
    }
  }
  return drafts;
}

// The tracker's half of the in-memory backend: invented foods, a fortnight of invented entries, and
// the commands a tracker view calls. It exists so the screens can be built and judged before any
// Rust is written (`docs/context/tools-plan-2026-10-11.md`, *the front end comes first*).
//
// Every number here is made up to look plausible. None of it is nutrition data.
import type {
  ObjectMeta,
  ViewInfo,
  TotalsDef,
  ViewGroup,
  DayTotals,
  LogDays,
  RecentItem,
  Nutrients,
  Product,
  Recipe,
  Routine,
  NoteDraft,
  FoodHit,
  FoodDetail,
  FoodPortion,
  PackStatus,
  Tracker,
  TrackerKind,
  HealthDay,
  HealthDays,
  HealthCount,
  HealthWeek,
  RecordItem,
  HealthRecord,
} from './types';
import { addDays, today, whole } from './units';

// ── the views a tracker ships with ───────────────────────────────────────────────────────────────

const FOOD: TotalsDef = {
  log: 'food',
  add: 'food',
  span: 'day',
  metrics: [
    { key: 'kcal', label: 'Energy', role: 'primary' },
    { key: 'protein_mg', label: 'Protein', role: 'secondary' },
    { key: 'carbs_mg', label: 'Carbs', role: 'secondary' },
    { key: 'fat_mg', label: 'Fat', role: 'secondary' },
    { key: 'fiber_mg', label: 'Fibre', role: 'detail' },
    { key: 'calcium_ug', label: 'Calcium', role: 'detail' },
    { key: 'iron_ug', label: 'Iron', role: 'detail' },
    { key: 'vitamin_c_ug', label: 'Vitamin C', role: 'detail' },
  ],
  slots: [
    { value: 'breakfast', label: 'Breakfast' },
    { value: 'lunch', label: 'Lunch' },
    { value: 'dinner', label: 'Dinner' },
    { value: 'snack', label: 'Snacks' },
  ],
};
const TRAINING: TotalsDef = {
  log: 'workout',
  add: 'workout',
  span: 'day',
  metrics: [
    { key: 'work_g', label: 'Volume load', role: 'primary' },
    { key: 'reps', label: 'Reps', role: 'secondary' },
  ],
  slots: [],
};

const HEALTH: TotalsDef = { log: 'health', add: 'health', span: 'diary', metrics: [], slots: [] };
const HEALTH_GROUP: ViewGroup = {
  name: 'Health',
  views: ['Diary', 'Over time', 'Guidance'],
};

const BODY: TotalsDef = {
  log: 'body',
  add: 'body',
  span: 'range',
  metrics: [{ key: 'weight_g', label: 'Weight', role: 'primary', agg: 'mean' }],
  slots: [],
};
const FOOD_GROUP: ViewGroup = { name: 'Food', views: ['Day', 'Menu', 'Trends', 'Diet'] };
const FITNESS_GROUP: ViewGroup = {
  name: 'Fitness',
  views: ['Week', 'Exercises', 'Regime'],
};

const DEFS: Record<string, { totals: TotalsDef; group: ViewGroup }> = {
  // In the order the `.view` files list them: a group opens on its first.
  Day: { totals: FOOD, group: FOOD_GROUP },
  Menu: { totals: { ...FOOD, span: 'menu' }, group: FOOD_GROUP },
  Trends: {
    totals: {
      ...FOOD,
      span: 'range',
      metrics: [
        ...FOOD.metrics,
        { key: 'weight_g', label: 'Weight', role: 'detail', agg: 'mean', log: 'body' },
      ],
    },
    group: FOOD_GROUP,
  },
  Diet: { totals: { ...FOOD, span: 'plan' }, group: FOOD_GROUP },
  Week: { totals: { ...TRAINING, span: 'week' }, group: FITNESS_GROUP },
  Exercises: { totals: { ...TRAINING, span: 'cards' }, group: FITNESS_GROUP },
  Regime: { totals: { ...TRAINING, span: 'plan' }, group: FITNESS_GROUP },
  Diary: { totals: HEALTH, group: HEALTH_GROUP },
  'Over time': { totals: { ...HEALTH, span: 'range' }, group: HEALTH_GROUP },
  Guidance: { totals: { ...HEALTH, span: 'plan' }, group: HEALTH_GROUP },
};

export const TRACKER_VIEWS: ViewInfo[] = Object.entries(DEFS).map(([name, d]) => ({
  name,
  renderer: 'totals',
  group_by: null,
  group: d.group.name,
  shipped: true,
}));

export const trackerDef = (name: string) => DEFS[name] ?? null;

/** The demo's targets, kept the way real ones are: on a note carrying `program: <log>`. */
const DEMO_TARGETS: Record<string, number> = {
  kcal: 2100,
  protein_mg: 140_000,
  carbs_mg: 230_000,
  fat_mg: 70_000,
  fiber_mg: 30_000,
  calcium_ug: 1_000_000,
  iron_ug: 8_000,
  vitamin_c_ug: 90_000,
};

/// The targets in force on a day, as the server finds them: the program for this log with the
/// latest `start` on or before the day — and, if it has a `due`, not past it — then the one
/// changed most recently.
function targetsOn(notes: ObjectMeta[], log: string, day: string, keys: string[]) {
  const programs = notes.filter(
    (x) =>
      x.props?.program === log &&
      !x.props.log &&
      !x.tags.includes('template') &&
      (!x.start || x.start <= day) &&
      (!x.due || x.due >= day),
  );
  programs.sort(
    (a, b) => (a.start ?? '').localeCompare(b.start ?? '') || a.updated.localeCompare(b.updated),
  );
  const p = programs.at(-1);
  const out: Record<string, number> = {};
  if (p) for (const k of keys) if (whole(p.props[k]) !== null) out[k] = whole(p.props[k])!;
  return out;
}

// ── invented foods, per 100 g ────────────────────────────────────────────────────────────────────

type Food = { id: string; name: string; v: number[]; portions: FoodPortion[] };
/** kcal, protein g, carbs g, fat g, fibre g, calcium mg, iron mg, vitamin C mg — per 100 g. */
const F = (id: string, name: string, v: number[], ...portions: [string, number][]): Food => ({
  id,
  name,
  v,
  portions: [
    ...portions.map(([label, grams]) => ({ label, grams })),
    { label: '100 g', grams: 100 },
  ],
});
const FOODS: Food[] = [
  F('f001', 'Oats, rolled', [379, 13.2, 67.7, 6.5, 10.1, 52, 4.3, 0], ['½ cup', 40], ['1 cup', 80]),
  F('f002', 'Milk, 2%', [50, 3.4, 4.9, 2, 0, 120, 0, 0], ['1 cup', 240], ['splash', 30]),
  F('f003', 'Egg, whole', [143, 12.6, 0.7, 9.5, 0, 56, 1.8, 0], ['1 large', 50], ['2 large', 100]),
  F(
    'f004',
    'Bread, wholemeal',
    [252, 12.5, 42.7, 3.5, 6, 161, 2.5, 0],
    ['1 slice', 32],
    ['2 slices', 64],
  ),
  F('f005', 'Butter', [717, 0.9, 0.1, 81, 0, 24, 0, 0], ['1 tsp', 5], ['1 tbsp', 14]),
  F('f006', 'Banana', [89, 1.1, 22.8, 0.3, 2.6, 5, 0.3, 8.7], ['1 medium', 118]),
  F('f007', 'Apple', [52, 0.3, 13.8, 0.2, 2.4, 6, 0.1, 4.6], ['1 medium', 182]),
  F('f008', 'Greek yoghurt, plain', [97, 9, 3.9, 5, 0, 100, 0, 0], ['1 pot', 150], ['1 cup', 245]),
  F(
    'f009',
    'Chicken breast, cooked',
    [165, 31, 0, 3.6, 0, 15, 1, 0],
    ['1 breast', 120],
    ['1 cup', 140],
  ),
  F(
    'f010',
    'Rice, white, cooked',
    [130, 2.7, 28.2, 0.3, 0.4, 10, 1.2, 0],
    ['1 cup', 158],
    ['½ cup', 79],
  ),
  F('f011', 'Rice, brown, cooked', [123, 2.7, 25.6, 1, 1.6, 3, 0.6, 0], ['1 cup', 195]),
  F('f012', 'Pasta, cooked', [158, 5.8, 30.9, 0.9, 1.8, 7, 1.3, 0], ['1 cup', 140]),
  F('f013', 'Olive oil', [884, 0, 0, 100, 0, 1, 0.6, 0], ['1 tsp', 5], ['1 tbsp', 14]),
  F('f014', 'Broccoli, cooked', [35, 2.4, 7.2, 0.4, 3.3, 40, 0.7, 64.9], ['1 cup', 156]),
  F(
    'f015',
    'Spinach, raw',
    [23, 2.9, 3.6, 0.4, 2.2, 99, 2.7, 28.1],
    ['1 cup', 30],
    ['handful', 25],
  ),
  F('f016', 'Tomato', [18, 0.9, 3.9, 0.2, 1.2, 10, 0.3, 13.7], ['1 medium', 123]),
  F('f017', 'Salmon, cooked', [206, 22.1, 0, 12.4, 0, 15, 0.3, 3.7], ['1 fillet', 150]),
  F('f018', 'Lentils, cooked', [116, 9, 20.1, 0.4, 7.9, 19, 3.3, 1.5], ['1 cup', 198]),
  F('f019', 'Chickpeas, cooked', [164, 8.9, 27.4, 2.6, 7.6, 49, 2.9, 1.3], ['1 cup', 164]),
  F('f020', 'Almonds', [579, 21.2, 21.6, 49.9, 12.5, 269, 3.7, 0], ['handful', 28]),
  F('f021', 'Peanut butter', [588, 25.1, 20, 50.4, 6, 43, 1.9, 0], ['1 tbsp', 16]),
  F(
    'f022',
    'Cheese, cheddar',
    [403, 22.9, 3.1, 33.3, 0, 710, 0.1, 0],
    ['1 slice', 21],
    ['30 g', 30],
  ),
  F('f023', 'Potato, boiled', [87, 1.9, 20.1, 0.1, 1.8, 5, 0.3, 13], ['1 medium', 173]),
  F('f024', 'Sweet potato, baked', [90, 2, 20.7, 0.2, 3.3, 38, 0.7, 19.6], ['1 medium', 130]),
  F('f025', 'Beef mince, cooked', [250, 26, 0, 15, 0, 18, 2.6, 0], ['1 patty', 113]),
  F('f026', 'Tofu, firm', [144, 17.3, 2.8, 8.7, 2.3, 683, 2.7, 0.2], ['½ block', 126]),
  F('f027', 'Avocado', [160, 2, 8.5, 14.7, 6.7, 12, 0.6, 10], ['½ fruit', 68], ['1 fruit', 136]),
  F('f028', 'Orange', [47, 0.9, 11.8, 0.1, 2.4, 40, 0.1, 53.2], ['1 medium', 131]),
  F('f029', 'Coffee, black', [1, 0.1, 0, 0, 0, 2, 0, 0], ['1 cup', 240]),
  F('f030', 'Tea, black', [1, 0, 0.3, 0, 0, 0, 0, 0], ['1 cup', 240]),
  F('f031', 'Orange juice', [45, 0.7, 10.4, 0.2, 0.2, 11, 0.2, 50], ['1 glass', 250]),
  F('f032', 'Honey', [304, 0.3, 82.4, 0, 0.2, 6, 0.4, 0.5], ['1 tsp', 7], ['1 tbsp', 21]),
  F(
    'f033',
    'Dark chocolate',
    [598, 7.8, 45.9, 42.6, 10.9, 73, 11.9, 0],
    ['1 square', 10],
    ['30 g', 30],
  ),
  F('f034', 'Tuna, canned in water', [116, 25.5, 0, 0.8, 0, 11, 1.5, 0], ['1 can', 142]),
  F('f035', 'Hummus', [166, 7.9, 14.3, 9.6, 6, 38, 2.4, 0], ['2 tbsp', 30]),
  F('f036', 'Carrot, raw', [41, 0.9, 9.6, 0.2, 2.8, 33, 0.3, 5.9], ['1 medium', 61]),
  F('f037', 'Cucumber', [15, 0.7, 3.6, 0.1, 0.5, 16, 0.3, 2.8], ['½ cucumber', 150]),
  F('f038', 'Blueberries', [57, 0.7, 14.5, 0.3, 2.4, 6, 0.3, 9.7], ['handful', 40], ['1 cup', 148]),
  F('f039', 'Granola', [471, 10, 64, 20, 7, 61, 3, 0], ['½ cup', 55]),
  F('f040', 'Pizza, margherita', [266, 11, 33, 10, 2.3, 188, 2.5, 1.4], ['1 slice', 107]),
];
const PACK = 'demo-foods@0';
const KEYS = [
  'kcal',
  'protein_mg',
  'carbs_mg',
  'fat_mg',
  'fiber_mg',
  'calcium_ug',
  'iron_ug',
  'vitamin_c_ug',
];
/** A food's row as stored values per 100 g: kcal as is, grams and milligrams ×1000. */
function per100(f: Food): Record<string, number> {
  return Object.fromEntries(KEYS.map((k, i) => [k, Math.round(i === 0 ? f.v[0] : f.v[i] * 1000)]));
}
export function valuesFor(per: Record<string, number>, grams: number): Record<string, number> {
  return Object.fromEntries(
    Object.entries(per).map(([k, v]) => [k, Math.round((v * grams) / 100)]),
  );
}

// The app carries its food list; the pretend one here is the forty invented foods above.
export function packStatus(): PackStatus {
  return {
    installed: {
      name: 'demo-foods',
      version: '0',
      title: 'Generic foods (pretend)',
      licence: 'Invented for the demo',
      attribution: 'Not nutrition data.',
      foods: FOODS.length,
    },
    bytes: 8_753_152,
  };
}

export function foodSearch(q: string, limit = 20): FoodHit[] {
  const words = q.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  return FOODS.filter((f) => words.every((w) => f.name.toLowerCase().includes(w)))
    .slice(0, limit)
    .map((f) => ({ id: f.id, name: f.name, src: 'demo', kcal: f.v[0] }));
}
export function foodGet(id: string): FoodDetail | null {
  const f = FOODS.find((x) => x.id === id.split('@')[0] || `${PACK}/${x.id}` === id);
  return f
    ? {
        id: f.id,
        name: f.name,
        src: 'demo',
        grp: '',
        per100: per100(f),
        portions: f.portions,
        pack: PACK,
      }
    : null;
}

// ── entries ──────────────────────────────────────────────────────────────────────────────────────

let n = 0;
function entry(title: string, start: string | null, props: Record<string, unknown>): ObjectMeta {
  n += 1;
  // Later entries are created later, so "order by slot, then created" has something to order by.
  const stamp = new Date(Date.UTC(2026, 0, 1) + n * 1000).toISOString();
  return {
    id: 'M0CKT' + String(n).padStart(21, '0'),
    type: 'note',
    title,
    preview: '',
    status: null,
    due: null,
    start,
    hard: false,
    created: stamp,
    updated: stamp,
    tags: [],
    assets: [],
    props,
    vault: 'personal',
  };
}
function foodEntry(day: string, slot: string, id: string, grams: number): ObjectMeta {
  const f = FOODS.find((x) => x.id === id)!;
  return entry(f.name, day, {
    log: 'food',
    slot,
    amount_g: grams,
    source: `${PACK}/${f.id}`,
    ...valuesFor(per100(f), grams),
  });
}

/** A few days' eating, repeated with small differences so a fortnight looks lived in. */
const MENU: [string, string, number][][] = [
  [
    ['breakfast', 'f001', 60],
    ['breakfast', 'f002', 200],
    ['breakfast', 'f006', 118],
    ['breakfast', 'f029', 240],
    ['lunch', 'f009', 150],
    ['lunch', 'f010', 200],
    ['lunch', 'f014', 156],
    ['lunch', 'f013', 10],
    ['dinner', 'f017', 150],
    ['dinner', 'f024', 180],
    ['dinner', 'f015', 60],
    ['snack', 'f008', 150],
    ['snack', 'f020', 28],
  ],
  [
    ['breakfast', 'f003', 100],
    ['breakfast', 'f004', 64],
    ['breakfast', 'f005', 10],
    ['breakfast', 'f029', 240],
    ['lunch', 'f018', 250],
    ['lunch', 'f004', 32],
    ['lunch', 'f016', 123],
    ['dinner', 'f012', 220],
    ['dinner', 'f025', 113],
    ['dinner', 'f022', 30],
    ['snack', 'f007', 182],
    ['snack', 'f033', 20],
  ],
  [
    ['breakfast', 'f008', 245],
    ['breakfast', 'f039', 55],
    ['breakfast', 'f038', 80],
    ['lunch', 'f034', 142],
    ['lunch', 'f004', 64],
    ['lunch', 'f027', 68],
    ['dinner', 'f026', 126],
    ['dinner', 'f011', 195],
    ['dinner', 'f014', 156],
    ['snack', 'f035', 60],
    ['snack', 'f036', 61],
  ],
];
const LIFTS: [string, number, number][] = [
  ['Squat', 60_000, 8],
  ['Bench press', 45_000, 8],
  ['Barbell row', 40_000, 10],
];
function setEntry(
  day: string,
  session: string,
  ex: string,
  set: number,
  reps: number,
  load: number,
  planned: boolean,
) {
  return entry(ex, day, {
    log: 'workout',
    session,
    exercise: ex,
    set,
    reps,
    load_g: load,
    ...(planned ? { planned: 1 } : { work_g: reps * load }),
  });
}

/** A fortnight of food, with today half eaten, and a few sessions with today's still to do. */
export function seedTracker(notes: ObjectMeta[]): void {
  const now = today();
  // What the days are measured against, in force since before the fortnight began.
  const program = entry('Food targets', addDays(now, -30), { program: 'food', ...DEMO_TARGETS });
  program.tags = ['program'];
  notes.push(program);
  for (let back = 13; back >= 0; back--) {
    const day = addDays(now, -back);
    // A weight most mornings, drifting down a little.
    if (back % 4 !== 1)
      notes.push(
        entry('Weight', day, {
          log: 'body',
          weight_g: 78_200 + back * 55 + ((back * 37) % 5) * 60,
        }),
      );
    if (back === 5) continue; // one day nobody logged, so "logged 13 of 14 days" is honest
    const menu = MENU[back % MENU.length];
    const meals = back === 0 ? menu.filter(([slot]) => slot === 'breakfast') : menu;
    for (const [slot, id, grams] of meals) {
      // A little variation in portions, fixed by the day so a reload shows the same fortnight.
      const g = Math.round(grams * (1 + (((back * 7 + grams) % 5) - 2) * 0.05));
      notes.push(foodEntry(day, slot, id, g));
    }
    if (back === 3) {
      // An entry typed by hand: energy only. It is what makes a nutrient total say "9 of 10".
      notes.push(entry('Takeaway curry', day, { log: 'food', slot: 'dinner', kcal: 720 }));
    }
    if (back % 3 === 0) {
      const session = `demo-session-${back}`;
      const grow = (13 - back) * 500; // a little heavier each session
      for (const [ex, load, reps] of LIFTS)
        for (let set = 1; set <= 3; set++)
          notes.push(
            setEntry(day, session, ex, set, set === 3 ? reps - 2 : reps, load + grow, back === 0),
          );
    }
  }
}

const isLog = (x: ObjectMeta, log: string) => x.props?.log === log;

export function logDays(
  notes: ObjectMeta[],
  args: { view: string; from: string; to: string; rows?: boolean },
): LogDays {
  const def = trackerDef(args.view)?.totals;
  if (!def) throw new Error(`no such view: ${args.view}`);
  const slotOrder = new Map(def.slots.map((s, i) => [s.value, i]));
  const days: DayTotals[] = [];
  for (let day = args.from; day <= args.to && days.length < 400; day = addDays(day, 1)) {
    const all = notes
      .filter((x) => isLog(x, def.log) && (x.start ?? '').slice(0, 10) === day)
      .sort(
        (a, b) =>
          (slotOrder.get(String(a.props.slot)) ?? 99) -
            (slotOrder.get(String(b.props.slot)) ?? 99) || a.created.localeCompare(b.created),
      );
    // A planned entry is not something that happened, so it is in no sum — but it is in the rows,
    // because the day's screen is where it gets confirmed.
    const counted = all.filter((x) => !x.props.planned);
    const sums: DayTotals['sums'] = {};
    for (const m of def.metrics) {
      let sum = 0;
      let carried = 0;
      // A value kept in another log is read from that log's entries of the day.
      const from = m.log
        ? notes.filter(
            (x) => isLog(x, m.log!) && (x.start ?? '').slice(0, 10) === day && !x.props.planned,
          )
        : counted;
      for (const x of from) {
        const v = whole(x.props[m.key]);
        if (v === null) continue;
        sum += v;
        carried += 1;
      }
      sums[m.key] = { sum, n: carried };
      // As `log.rs`: an amount given as a range scales every value of its entry.
      const scaled = (which: 'amount_min_g' | 'amount_max_g') =>
        from.reduce((t, x) => {
          const v = whole(x.props[m.key]);
          if (v === null) return t;
          const [a, b] = [whole(x.props.amount_g), whole(x.props[which])];
          return t + (a && b ? Math.floor((v * b) / a) : v);
        }, 0);
      if (from.some((x) => whole(x.props.amount_min_g) && whole(x.props.amount_max_g)))
        Object.assign(sums[m.key], { low: scaled('amount_min_g'), high: scaled('amount_max_g') });
    }
    const at = counted
      .map((x) => (x.start ?? '').slice(11, 16))
      .filter(Boolean)
      .sort();
    const mins = at.map((t) => Number(t.slice(0, 2)) * 60 + Number(t.slice(3)));
    days.push({
      day,
      of: counted.length,
      ...(at.length
        ? {
            times: {
              occasions: 1 + mins.filter((m, i) => i > 0 && m - mins[i - 1] >= 15).length,
              first: at[0],
              last: at.at(-1)!,
            },
          }
        : {}),
      sums,
      targets: targetsOn(
        notes,
        def.log,
        day,
        def.metrics.map((m) => m.key),
      ),
      ...(args.rows ? { rows: all.map((x) => JSON.parse(JSON.stringify(x)) as ObjectMeta) } : {}),
    });
  }
  return { days };
}

export function logRecent(
  notes: ObjectMeta[],
  args: { view: string; slot?: string; q?: string; limit?: number },
): RecentItem[] {
  const def = trackerDef(args.view)?.totals;
  if (!def) throw new Error(`no such view: ${args.view}`);
  const q = (args.q ?? '').toLowerCase().trim();
  const seen = new Map<string, RecentItem & { inSlot: number }>();
  for (const x of notes) {
    if (!isLog(x, def.log) || !x.start || x.props.planned) continue;
    const title = x.title ?? '';
    if (q && !title.toLowerCase().includes(q)) continue;
    const key = String(x.props.source ?? title.toLowerCase());
    const had = seen.get(key);
    const inSlot = args.slot && x.props.slot === args.slot ? 1 : 0;
    if (!had) {
      seen.set(key, { title, props: { ...x.props }, count: 1, last: x.start, inSlot });
    } else {
      had.count += 1;
      had.inSlot += inSlot;
      if (x.start >= had.last) {
        had.last = x.start;
        had.props = { ...x.props };
      }
    }
  }
  return [...seen.values()]
    .sort((a, b) => b.inSlot - a.inSlot || b.count - a.count || b.last.localeCompare(a.last))
    .slice(0, args.limit ?? 12)
    .map(({ inSlot: _inSlot, ...rest }) => rest);
}

/** A stand-in: three rows, one figure each. The table and its sources are `nutrients.rs`. */
export function logNutrients(notes: ObjectMeta[], args: { view: string; day: string }): Nutrients {
  const def = trackerDef(args.view)?.totals;
  if (!def) throw new Error(`no such view: ${args.view}`);
  const entries = notes.filter(
    (x) => isLog(x, def.log) && (x.start ?? '').slice(0, 10) === args.day && !x.props.planned,
  );
  const row = (key: string, label: string, reference: number | undefined, kind: string) => {
    const got = entries.map((x) => whole(x.props[key])).filter((v): v is number => v !== null);
    return { key, label, sum: got.reduce((t, v) => t + v, 0), n: got.length, reference, kind };
  };
  return {
    day: args.day,
    of: entries.length,
    needs_sex: true,
    entries,
    rows: [
      row('fiber_mg', 'Fibre', 25_000, 'AI'),
      row('vitamin_c_ug', 'Vitamin C', undefined, 'PRI'),
      row('calcium_ug', 'Calcium', 950_000, 'PRI'),
    ],
  };
}
export function logProducts(notes: ObjectMeta[]): Product[] {
  return notes
    .filter((x) => x.props.product && !x.props.log && !x.tags.includes('template'))
    .map((x) => ({
      id: x.id,
      title: x.title ?? '',
      barcode: x.props.barcode ? String(x.props.barcode) : undefined,
      serving_g: whole(x.props.serving_g) ?? undefined,
      serving_ml: whole(x.props.serving_ml) ?? undefined,
      unit: x.props.unit === 'ml' ? 'ml' : undefined,
      per100: Object.fromEntries(
        Object.entries(x.props).flatMap(([k, v]) =>
          (k === 'kcal' || /_(mg|ug)$/.test(k)) && typeof v === 'number' ? [[k, v]] : [],
        ),
      ),
    }))
    .sort((a, b) => a.title.localeCompare(b.title));
}
export function logRoutines(notes: ObjectMeta[], args: { view: string }): Routine[] {
  const def = trackerDef(args.view)?.totals;
  if (!def) throw new Error(`no such view: ${args.view}`);
  const named = notes.filter((x) => !x.start && x.props.routine && !x.tags.includes('template'));
  return named
    .filter((x) => !x.props.log)
    .map((head) => ({
      routine: String(head.props.routine),
      id: head.id,
      title: head.title ?? '',
      weekdays: String(head.props.weekdays ?? '')
        .split(',')
        .map((d) => d.trim().toLowerCase())
        .filter(Boolean),
      sets: named.filter((x) => isLog(x, def.log) && x.props.routine === head.props.routine),
    }))
    .sort((a, b) => a.title.localeCompare(b.title));
}
export function logRecipes(notes: ObjectMeta[], args: { view: string }): Recipe[] {
  const def = trackerDef(args.view)?.totals;
  if (!def) throw new Error(`no such view: ${args.view}`);
  // A recipe's own note is an ordinary note (no `log`); its ingredients are entries of this log.
  const saved = notes.filter(
    (x) =>
      (isLog(x, def.log) || !x.props.log) &&
      !x.start &&
      x.props.recipe &&
      !x.tags.includes('template'),
  );
  const whole = (v: unknown) => (typeof v === 'number' && Number.isInteger(v) ? v : null);
  return saved
    .filter((x) => (whole(x.props.servings) ?? 0) >= 1)
    .map((head) => {
      const ingredients = saved.filter(
        (x) =>
          x.props.recipe === head.props.recipe &&
          whole(x.props.servings) === null &&
          isLog(x, def.log),
      );
      const sums: Recipe['sums'] = {};
      for (const x of ingredients)
        for (const [k, v] of Object.entries(x.props)) {
          const n = whole(v);
          if (n === null) continue;
          sums[k] ??= { sum: 0, n: 0 };
          sums[k].sum += n;
          sums[k].n += 1;
        }
      return {
        recipe: String(head.props.recipe),
        id: head.id,
        title: head.title ?? '',
        servings: whole(head.props.servings) ?? 1,
        of: ingredients.length,
        sums,
        ingredients: ingredients.map((x) => JSON.parse(JSON.stringify(x)) as ObjectMeta),
      };
    })
    .sort((a, b) => a.title.toLowerCase().localeCompare(b.title.toLowerCase()));
}

export function createNotes(notes: ObjectMeta[], drafts: NoteDraft[]): ObjectMeta[] {
  const made = drafts.map((d) => {
    const { due, ...rest } = d.props;
    const x = entry(d.title, d.start ?? null, rest);
    if (typeof due === 'string' && due) x.due = due;
    x.created = x.updated = new Date().toISOString();
    x.tags = d.tags ?? [];
    return x;
  });
  notes.push(...made);
  return made.map((x) => JSON.parse(JSON.stringify(x)) as ObjectMeta);
}

/** Set several properties in one write. An empty string removes one. Answers what each was. */
export function setProperties(
  notes: ObjectMeta[],
  id: string,
  props: Record<string, string | number>,
): { previous: Record<string, string | null> } {
  const x = notes.find((y) => y.id === id);
  if (!x) throw new Error(`no such note: ${id}`);
  if ('log' in props)
    throw new Error('`log` is what kind of entry this is — it is set when the entry is made');
  // As the server answers: the value as text, or null when there was none.
  const previous: Record<string, string | null> = {};
  for (const [k, v] of Object.entries(props)) {
    const was =
      k === 'title'
        ? x.title
        : k === 'start'
          ? x.start
          : k === 'due'
            ? x.due
            : (x.props[k] as string | number | undefined);
    previous[k] = was === null || was === undefined ? null : String(was);
    if (k === 'title') x.title = String(v) || null;
    else if (k === 'start') x.start = String(v) || null;
    else if (k === 'due') x.due = String(v) || null;
    else if (v === '') delete x.props[k];
    // Typed as the server types it: text that is a whole number is stored as one.
    else x.props[k] = typeof v === 'string' && /^-?\d+$/.test(v) ? Number(v) : v;
  }
  x.updated = new Date().toISOString();
  return { previous };
}

// ── the Health diary (`health.rs`; `decisions.md#ui`, 2026-10-22) ────────────────────────────────

/** As `health::parse_key`: what kind a tracker is, read off the end of its key. */
const KIND_SUFFIXES: [string, TrackerKind][] = [
  ['_0to3', { kind: 'severity' }],
  ['_0to10', { kind: 'rating' }],
  ['_0to100', { kind: 'bother' }],
  ['_yes', { kind: 'yesno' }],
  ['_count', { kind: 'count' }],
  ['_doses', { kind: 'count' }],
  ['_score', { kind: 'score' }],
];
const UNITS = ['min', 'ml', 'mg', 'g', 'mmhg', 'pct', 'c', 'c10', 'mmoll10'];
const STEM = /^[a-z][a-z0-9_-]{0,31}$/;
const named = (stem: string) => STEM.test(stem) && !stem.endsWith('_');

export function parseTrackerKey(key: string): { stem: string; kind: TrackerKind } | null {
  for (const [suffix, kind] of KIND_SUFFIXES)
    if (key.endsWith(suffix) && named(key.slice(0, -suffix.length)))
      return { stem: key.slice(0, -suffix.length), kind };
  const at = key.lastIndexOf('_');
  if (at < 0) return null;
  const [stem, unit] = [key.slice(0, at), key.slice(at + 1)];
  return UNITS.includes(unit) && named(stem) ? { stem, kind: { kind: 'amount', unit } } : null;
}

const BOUNDS: Partial<Record<TrackerKind['kind'], [number, number]>> = {
  severity: [0, 3],
  rating: [0, 10],
  bother: [0, 100],
  yesno: [0, 1],
  count: [0, Number.MAX_SAFE_INTEGER],
};

export function healthTrackers(notes: ObjectMeta[]): Tracker[] {
  const found = new Map<string, Tracker>();
  for (const n of notes) {
    if (!n.tags.includes('template') || n.props.group !== 'Health') continue;
    for (const key of Object.keys(n.props)) {
      const p = parseTrackerKey(key);
      if (!p || found.has(key)) continue;
      const words = p.stem.replace(/_/g, ' ');
      found.set(key, { key, label: words.charAt(0).toUpperCase() + words.slice(1), ...p.kind });
    }
  }
  return [...found.values()].sort((a, b) => (a.key < b.key ? -1 : 1));
}

export function healthDays(
  notes: ObjectMeta[],
  args: { from: string; to: string; weeks?: boolean },
): HealthDays {
  const trackers = healthTrackers(notes);
  const entries = notes
    .filter((x) => isLog(x, 'health'))
    .sort((a, b) => a.created.localeCompare(b.created));
  const days: HealthDay[] = [];
  const counts: Record<string, HealthCount> = Object.fromEntries(
    trackers.map((t) => [
      t.key,
      {
        of: 0,
        recorded: 0,
        days: 0,
        ...(t.kind === 'severity' ? { moderate_or_severe: 0 } : {}),
      },
    ]),
  );
  for (let day = args.from; day <= args.to && days.length < 400; day = addDays(day, 1)) {
    const here = entries.filter((x) => (x.start ?? '').slice(0, 10) === day);
    const values: Record<string, number> = {};
    for (const t of trackers) {
      const c = counts[t.key];
      c.of += 1;
      const b = BOUNDS[t.kind];
      for (const x of here) {
        if (!(t.key in x.props)) continue;
        const v = whole(x.props[t.key]);
        if (v === null || (b && (v < b[0] || v > b[1]))) {
          c.odd = (c.odd ?? 0) + 1;
          continue;
        }
        const had = values[t.key];
        values[t.key] =
          had === undefined
            ? v
            : t.kind === 'count' || t.kind === 'amount'
              ? had + v
              : Math.max(had, v);
      }
      const v = values[t.key];
      if (v === undefined) continue;
      c.recorded += 1;
      if (v >= 1) c.days += 1;
      if (v >= 2 && c.moderate_or_severe !== undefined) c.moderate_or_severe += 1;
    }
    // As `health::elsewhere`: Food's water, caffeine and alcohol summed; the body log's weight a
    // mean and its steps summed, for the day.
    const elsewhere: Record<string, number> = {};
    const onDay = (log: string) =>
      notes.filter(
        (x) => isLog(x, log) && (x.start ?? '').slice(0, 10) === day && !x.props.planned,
      );
    for (const k of ['water_mg', 'caffeine_ug', 'alcohol_mg']) {
      const vs = onDay('food')
        .map((x) => whole(x.props[k]))
        .filter((v) => v !== null) as number[];
      if (vs.length) elsewhere[k] = vs.reduce((a, b) => a + b, 0);
    }
    const weights = onDay('body')
      .map((x) => whole(x.props.weight_g))
      .filter((v) => v !== null) as number[];
    if (weights.length)
      elsewhere.weight_g = Math.floor(weights.reduce((a, b) => a + b, 0) / weights.length);
    const steps = onDay('body')
      .map((x) => whole(x.props.steps))
      .filter((v) => v !== null) as number[];
    if (steps.length) elsewhere.steps = steps.reduce((a, b) => a + b, 0);
    const first = here[0];
    const context = here
      .map((x) => x.props.cycle_context)
      .find((c) => typeof c === 'string' && c.trim());
    days.push({
      day,
      ...(first ? { id: first.id } : {}),
      ...(typeof context === 'string' ? { context: context.trim() } : {}),
      ...(Object.keys(elsewhere).length ? { elsewhere } : {}),
      values,
      ...(first && first.created.slice(0, 10) > addDays(day, 1) ? { late: true } : {}),
    });
  }
  const template =
    notes
      .filter((n) => n.tags.includes('template') && n.props.group === 'Health')
      .sort((a, b) => a.created.localeCompare(b.created))[0]?.id ?? null;
  const out: HealthDays = { template, trackers, days, counts };
  if (args.weeks) {
    // As `fm_query::days::per_week`: ISO weeks from Monday, a part week counting only its days.
    out.weeks = Object.fromEntries(
      trackers.map((t) => {
        const weeks: HealthWeek[] = [];
        for (const d of days) {
          const dow = (new Date(`${d.day}T12:00:00`).getDay() + 6) % 7;
          const week = addDays(d.day, -dow);
          let w = weeks[weeks.length - 1];
          if (!w || w.week !== week)
            weeks.push((w = { week, of: 0, recorded: 0, days: 0, sum: 0 }));
          w.of += 1;
          const v = d.values[t.key];
          if (v === undefined) continue;
          w.recorded += 1;
          w.sum += v;
          if (v >= 1) w.days += 1;
        }
        return [t.key, weeks];
      }),
    );
  }
  return out;
}

/** As `record::health_record`: every note (not a template) with a `record` kind, in kind order. */
const RECORD_KINDS = [
  'medicine',
  'allergy',
  'condition',
  'result',
  'visit',
  'immunization',
  'procedure',
  'device',
  'contact',
  'none_known',
];
export function healthRecord(notes: ObjectMeta[]): HealthRecord {
  const own = new Set(['record', 'record_status', 'about', 'shelf']);
  const templates = [
    ...new Set(
      notes
        .filter((n) => n.tags.includes('template') && typeof n.props.record === 'string')
        .map((n) => String(n.props.record)),
    ),
  ].sort();
  const items = notes
    .filter(
      (n) =>
        !n.tags.includes('template') &&
        typeof n.props.record === 'string' &&
        RECORD_KINDS.includes(n.props.record),
    )
    .map((n) => ({
      id: n.id,
      kind: n.props.record as RecordItem['kind'],
      title: n.title ?? '',
      ...(typeof n.props.record_status === 'string' ? { status: n.props.record_status } : {}),
      ...(n.start ? { start: n.start.slice(0, 10) } : {}),
      ...(n.due ? { due: n.due.slice(0, 10) } : {}),
      ...(typeof n.props.about === 'string' ? { about: n.props.about } : {}),
      fields: Object.fromEntries(
        Object.entries(n.props)
          .filter(([k, v]) => !own.has(k) && String(v).trim())
          .map(([k, v]) => [k, String(v)]),
      ),
    }))
    .sort(
      (a, b) =>
        RECORD_KINDS.indexOf(a.kind) - RECORD_KINDS.indexOf(b.kind) ||
        a.title.localeCompare(b.title),
    );
  return { items, templates };
}

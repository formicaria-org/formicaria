// A packet's label, as it is printed, turned into what a food is kept as: whole numbers per 100
// of its own unit (`decisions.md#data`, 2026-10-13, *a label is typed as it is printed*).
//
// A European label is per 100 g or per 100 ml, in kJ and kcal, with salt in grams. An American
// one is per serving, in kcal, with sodium in milligrams. Singapore allows either
// (`sources-2026-10-11.md` §6f). The form asks which; this does the arithmetic, and only this.

/** What the label's figures are for. `each`: one piece — a tablet, a capsule, a drop — as a
 *  supplement's label gives them (2026-10-31). */
export type Basis = '100g' | '100ml' | 'serving' | 'each';
export type Unit = 'g' | 'ml' | 'each';

export type LabelForm = {
  basis: Basis;
  /** The size of a serving and what it is measured in. Needed when the figures are per serving. */
  serving: number | string | null;
  servingUnit: Unit;
  energyIn: 'kcal' | 'kJ';
  sodiumAs: 'salt' | 'sodium';
  /** What was typed, by the key it is kept under. */
  values: Record<string, number | string | null | undefined>;
  /** What one piece is, when the figures are per piece: `tablet`, `capsule` (`amount.ts` `PIECES`). */
  piece?: string;
  /** The vitamins and minerals added to the form, in the order added, each in the unit it was
   *  typed in as the label prints it: µg or mg. Their figures are in `values`, by `key`. */
  micros?: { key: string; unit: 'µg' | 'mg' }[];
};

/** **Vitamins and minerals a label may give** (2026-10-31: a supplement's whole label is these):
 *  every value of the nutrients sheet a label prints, by the key the app keeps it under, with the
 *  unit labels usually print it in — the person may change it to the one their label uses.
 *  `factor` turns what is printed into the key's own definition where they differ: a label's
 *  *folic acid* is 1.7 dietary folate equivalents each (DFE = food folate + 1.7 × folic acid, the
 *  definition the food list keeps, `ci/pack-foods.py`). */
export const MICROS: { key: string; label: string; unit: 'µg' | 'mg'; factor?: number }[] = [
  { key: 'vitamin_a_rae_ug', label: 'Vitamin A', unit: 'µg' },
  { key: 'vitamin_d_ng', label: 'Vitamin D', unit: 'µg' },
  { key: 'vitamin_e_ug', label: 'Vitamin E', unit: 'mg' },
  { key: 'vitamin_k1_ug', label: 'Vitamin K', unit: 'µg' },
  { key: 'vitamin_c_ug', label: 'Vitamin C', unit: 'mg' },
  { key: 'thiamin_ug', label: 'Thiamin (B1)', unit: 'mg' },
  { key: 'riboflavin_ug', label: 'Riboflavin (B2)', unit: 'mg' },
  { key: 'niacin_ug', label: 'Niacin (B3)', unit: 'mg' },
  { key: 'pantothenic_acid_ug', label: 'Pantothenic acid (B5)', unit: 'mg' },
  { key: 'vitamin_b6_ug', label: 'Vitamin B6', unit: 'mg' },
  { key: 'folate_dfe_ug', label: 'Folic acid', unit: 'µg', factor: 1.7 },
  { key: 'vitamin_b12_ng', label: 'Vitamin B12', unit: 'µg' },
  { key: 'choline_ug', label: 'Choline', unit: 'mg' },
  { key: 'calcium_ug', label: 'Calcium', unit: 'mg' },
  { key: 'magnesium_ug', label: 'Magnesium', unit: 'mg' },
  { key: 'iron_ug', label: 'Iron', unit: 'mg' },
  { key: 'zinc_ug', label: 'Zinc', unit: 'mg' },
  { key: 'potassium_ug', label: 'Potassium', unit: 'mg' },
  { key: 'phosphorus_ug', label: 'Phosphorus', unit: 'mg' },
  { key: 'copper_ug', label: 'Copper', unit: 'mg' },
  { key: 'manganese_ug', label: 'Manganese', unit: 'mg' },
  { key: 'selenium_ug', label: 'Selenium', unit: 'µg' },
  { key: 'iodine_ug', label: 'Iodine', unit: 'µg' },
  { key: 'caffeine_ug', label: 'Caffeine', unit: 'mg' },
];
/** How many of a key's own unit one printed unit is: `vitamin_d_ng` typed in µg is 1,000 ng. */
export function perPrinted(key: string, printed: 'µg' | 'mg'): number {
  const kept = key.endsWith('_ng') ? 1e-9 : key.endsWith('_ug') ? 1e-6 : 1e-3;
  return Math.round((printed === 'µg' ? 1e-6 : 1e-3) / kept);
}

/** One kilocalorie in kilojoules. */
export const KJ_PER_KCAL = 4.184;

/** The lines of a label, in the order a European one prints them. `per` is how many kept units
 *  one typed unit is: grams are kept as milligrams. Energy and sodium are asked with a unit. */
export const LINES: { key: string; label: string; per: number }[] = [
  { key: 'fat_mg', label: 'Fat (g)', per: 1000 },
  { key: 'saturated_fat_mg', label: 'of which saturates (g)', per: 1000 },
  { key: 'carbs_mg', label: 'Carbohydrate (g)', per: 1000 },
  { key: 'sugars_mg', label: 'of which sugars (g)', per: 1000 },
  { key: 'fiber_mg', label: 'Fibre (g)', per: 1000 },
  { key: 'protein_mg', label: 'Protein (g)', per: 1000 },
];
export const ENERGY = 'kcal';
export const SODIUM = 'sodium_ug';

const typed = (v: number | string | null | undefined): number | null => {
  if (v === '' || v === null || v === undefined) return null;
  const n = Number(v);
  return Number.isFinite(n) && n >= 0 ? n : null;
};

/** The unit the food is measured in: what its figures are per 100 of. */
export function unitOf(form: Pick<LabelForm, 'basis' | 'servingUnit'>): Unit {
  if (form.basis === '100ml') return 'ml';
  if (form.basis === 'each') return 'each';
  if (form.basis === 'serving') return form.servingUnit;
  return 'g';
}

/** The size of the serving, when one was given. */
export function servingOf(form: Pick<LabelForm, 'serving'>): number | null {
  const n = Math.round(Number(form.serving));
  return Number.isFinite(n) && n > 0 ? n : null;
}

/** The label's figures per 100 of the food's unit, as they are kept — or `null` when they are
 *  per serving and no serving size was given, which is the one thing that cannot be worked out. */
export function per100(form: LabelForm): Record<string, number> | null {
  let scale = 1;
  if (form.basis === 'serving') {
    const s = servingOf(form);
    if (s === null) return null;
    scale = 100 / s;
  }
  // Per piece, kept per 100 pieces like every food is per 100 of its unit: one arithmetic for all.
  if (form.basis === 'each') scale = 100;
  const out: Record<string, number> = {};
  const energy = typed(form.values[ENERGY]);
  if (energy !== null)
    out[ENERGY] = Math.round((form.energyIn === 'kJ' ? energy / KJ_PER_KCAL : energy) * scale);
  for (const l of LINES) {
    const v = typed(form.values[l.key]);
    if (v !== null) out[l.key] = Math.round(v * l.per * scale);
  }
  const sodium = typed(form.values[SODIUM]);
  // Salt is sodium × 2.5 (Regulation (EU) 1169/2011, Annex I): a gram of salt is 400 mg of sodium.
  if (sodium !== null)
    out[SODIUM] = Math.round(sodium * (form.sodiumAs === 'salt' ? 400_000 : 1000) * scale);
  for (const m of form.micros ?? []) {
    const v = typed(form.values[m.key]);
    const line = MICROS.find((x) => x.key === m.key);
    if (v !== null && line)
      out[m.key] = Math.round(v * perPrinted(m.key, m.unit) * (line.factor ?? 1) * scale);
  }
  return out;
}

/** Whether the form has any figure at all: a supplement's label may give no energy. */
export const anyFigure = (kept: Record<string, number>) => Object.keys(kept).length > 0;

/** The figures of a food kept per 100, as a European label would print them: what a look-up
 *  fills the form with. */
export function typedFrom(kept: Record<string, number>, per = 1): LabelForm['values'] {
  const out: LabelForm['values'] = {};
  const two = (n: number) => Math.round(n * 100) / 100;
  if (kept[ENERGY] !== undefined) out[ENERGY] = two(kept[ENERGY] / per);
  for (const l of LINES) if (kept[l.key] !== undefined) out[l.key] = two(kept[l.key] / l.per / per);
  if (kept[SODIUM] !== undefined) out[SODIUM] = two(kept[SODIUM] / 400_000 / per);
  for (const m of microsOf(kept))
    out[m.key] = two(
      kept[m.key] /
        perPrinted(m.key, m.unit) /
        (MICROS.find((x) => x.key === m.key)?.factor ?? 1) /
        per,
    );
  return out;
}

/** The vitamin and mineral lines a kept food has, each in the unit labels usually print it in. */
export const microsOf = (kept: Record<string, number>): NonNullable<LabelForm['micros']> =>
  MICROS.filter((m) => kept[m.key] !== undefined).map((m) => ({ key: m.key, unit: m.unit }));

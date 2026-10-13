// A packet's label, as it is printed, turned into what a food is kept as: whole numbers per 100
// of its own unit (`decisions.md#data`, 2026-10-13, *a label is typed as it is printed*).
//
// A European label is per 100 g or per 100 ml, in kJ and kcal, with salt in grams. An American
// one is per serving, in kcal, with sodium in milligrams. Singapore allows either
// (`sources-2026-10-11.md` §6f). The form asks which; this does the arithmetic, and only this.

/** What the label's figures are for. */
export type Basis = '100g' | '100ml' | 'serving';
export type Unit = 'g' | 'ml';

export type LabelForm = {
  basis: Basis;
  /** The size of a serving and what it is measured in. Needed when the figures are per serving. */
  serving: number | string | null;
  servingUnit: Unit;
  energyIn: 'kcal' | 'kJ';
  sodiumAs: 'salt' | 'sodium';
  /** What was typed, by the key it is kept under. */
  values: Record<string, number | string | null | undefined>;
};

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
  return out;
}

/** The figures of a food kept per 100, as a European label would print them: what a look-up
 *  fills the form with. */
export function typedFrom(kept: Record<string, number>): LabelForm['values'] {
  const out: LabelForm['values'] = {};
  const two = (n: number) => Math.round(n * 100) / 100;
  if (kept[ENERGY] !== undefined) out[ENERGY] = kept[ENERGY];
  for (const l of LINES) if (kept[l.key] !== undefined) out[l.key] = two(kept[l.key] / l.per);
  if (kept[SODIUM] !== undefined) out[SODIUM] = two(kept[SODIUM] / 400_000);
  return out;
}

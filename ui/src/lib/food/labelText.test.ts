import { describe, expect, it } from 'vitest';
import { readLabel } from './labelText';
import { per100, type LabelForm } from './label';

// The four transcriptions below marked "as read" are what qwen3-vl-4b returned on 2026-10-13
// for photographs from Open Food Facts, unedited; the rest are written to the shapes labels take.

const COLA = `| Per: | 100ml | 330ml (*) |
|---|---|---|
| Energy: | 180kJ/42kcal | 594kJ/139kcal (7%) |
| Fat: | 0g | 0g (0%) |
| of which saturates: | 0g | 0g (0%) |
| Carbohydrate: | 10.6g | 35g (13%) |
| of which sugars: | 10.6g | 35g (39%) |
| Protein: | 0g | 0g (0%) |
| Salt: | 0g | 0g (0%) |`;

const ALMONDS = `| Typical values | per 100g | per serving 30g | %RI* |
|---|---|---|---|
| Energy | 2567kJ/621kcal | 770kJ/186kcal | 9% |
| Fat | 53.3g | 16.0g | 23% |
| of which saturates | 4.3g | 1.3g | 7% |
| Carbohydrate | 4.8g | 1.4g | <1% |
| of which sugars | 4.8g | 1.4g | 2% |
| Fibre | 12.1g | 3.6g |  |
| Protein | 24.5g | 7.4g | 15% |
| Salt | 0.01g | <0.01g | <1% |
| *Reference intake of an average adult (8400kJ/2000kcal) |  |  |  |
| Minerals | per 100g %NRV | per serving 30g %NRV |  |
| Magnesium | 260mg 69% | 78mg 21% |  |
| NRV = Nutrient reference values |  |  |  |
| This pack contains approx. 6 servings |  |  |  |`;

const FRONT_OF_JAR = `| | 15g | 100g |
|---|---|---|
| Energy | 339 kJ / 81 kcal | 2278 kJ / 546 kcal |
| Fat | 4%* | [?] |
| | | [?] |`;

// Two tables printed side by side, which the reader ran together: the first columns of the
// first lines are right, and nothing in the figures says the rest is not.
const TWO_TABLES = `| |100 g*|80 g|%VD*|100 g*|80 g|%VD*|
|---|---|---|---|---|---|---|
|Valor energético (kcal)|170|287|14|0,9|1,6|2|
|Carboidratos (g)|33|57|19|0,2|0,4|2|
|Açúcares totais (g)|1,6|2,8|0|0|0|10|
|Açúcares adicionados (g)|0|0|0|1,4|2,4|0|
|Proteínas (g)|6|10|20|2,3|4|0|
|Gorduras totais (g)|0,9|1,6|2|0,2|0,4|2|
|Gorduras saturadas (g)|0,2|0,4|2|0|0|0|
|Gorduras trans (g)|0|0|0|1,4|2,4|0|
|Fibras alimentares (g)|2,3|4|0|2,3|4|0|
|Sódio (mg)|2,3|4|0|2,3|4|0|`;

// Two printed lines in one row, parted by a line break; a piece of 21.5 g.
const TWO_IN_A_ROW = `| VALEURS NUTRITIONNELLES MOYENNES | POUR 100 g | PAR PIECE 21,5 g | PAR 21,5 g |
|---|---|---|---|
| Energie | 2384 kJ / 572 kcal | 510 kJ / 122 kcal | 6 |
| Matières grasses<br>dont acides gras saturés | 37,3 g<br>17,3 g | 8 g<br>3,7 g | 11<br>19 |
| Glucides<br>dont sucres | 49,5 g<br>41,2 g | 10,6 g<br>8,9 g | 4<br>10 |
| Protéines | 8,6 g | 1,8 g | 4 |
| Sel | 0,272 g | 0,058 g | 1 |
| *Apport de référence pour un adulte-type (8 400 kJ/2 000 kcal) |  |  |  |`;

// Three languages to a line, and every unit printed before its figure.
const UNITS_FIRST = `| Déclaration nutritionnelle - Nährwertdeklaration - Voedingswaarden | g 100 |
| --- | --- |
| Énergie - Energie - Energie | kJ2030 |
|  | kcal 492 |
| Matières grasses - Fett - Vetten | g 47 |
| dont acides gras saturés - davon gesättigte Fettsäuren - waarvan verzadigde vetzuren | g 5,3 |
| Glucides - Kohlenhydrate - Koolhydraten | g 11 |
| dont sucres - davon Zucker - waarvan suikers | g 5,0 |
| Fibres alimentaires - Ballaststoffe - Vezel | g 3,0 |
| Protéines - Eiweiß - Eiwitten | g 4,7 |
| Sel - Salz - Zout | g 3,2 |
| GLUTENFREI / SANS GLUTEN / GLUTENVRIJ |  |`;

const form = (r: ReturnType<typeof readLabel>): LabelForm => ({
  basis: '100g',
  serving: null,
  servingUnit: 'g',
  energyIn: 'kcal',
  sodiumAs: 'salt',
  ...r.form,
});

describe('a label read out of its transcription', () => {
  it('reads a drink per 100 ml and keeps the can as its serving (as read)', () => {
    const r = readLabel(COLA);
    expect(r.form.basis).toBe('100ml');
    expect(r.form.serving).toBe(330);
    expect(r.form.energyIn).toBe('kcal');
    expect(r.form.values).toEqual({
      kcal: 42,
      fat_mg: 0,
      saturated_fat_mg: 0,
      carbs_mg: 10.6,
      sugars_mg: 10.6,
      protein_mg: 0,
      sodium_ug: 0,
    });
    expect(r.doubts).toEqual([]);
    expect(per100(form(r))).toMatchObject({ kcal: 42, carbs_mg: 10_600, sugars_mg: 10_600 });
  });

  it('takes the per 100 g column and not the serving beside it (as read)', () => {
    const r = readLabel(ALMONDS);
    expect(r.form.basis).toBe('100g');
    expect(r.form.serving).toBe(30);
    expect(r.form.values).toEqual({
      kcal: 621,
      fat_mg: 53.3,
      saturated_fat_mg: 4.3,
      carbs_mg: 4.8,
      sugars_mg: 4.8,
      fiber_mg: 12.1,
      protein_mg: 24.5,
      sodium_ug: 0.01,
    });
    expect(r.form.sodiumAs).toBe('salt');
    expect(r.doubts).toEqual([]);
  });

  it('leaves empty what the reader could not make out (as read)', () => {
    const r = readLabel(FRONT_OF_JAR);
    expect(r.form.values).toEqual({ kcal: 546 });
    expect(r.form.basis).toBe('100g');
  });

  it('reads a unit named once in the line, and sodium in milligrams (as read)', () => {
    const r = readLabel(TWO_TABLES);
    expect(r.form.basis).toBe('100g');
    expect(r.form.values).toMatchObject({
      kcal: 170,
      carbs_mg: 33,
      sugars_mg: 1.6,
      protein_mg: 6,
      fat_mg: 0.9,
      saturated_fat_mg: 0.2,
    });
    expect(r.form.sodiumAs).toBe('sodium');
    // Added sugars and trans fat are other figures and are not taken for sugars or fat.
    expect(r.form.values.sugars_mg).toBe(1.6);
  });

  it('parts two lines a reader put in one row (as read)', () => {
    const r = readLabel(TWO_IN_A_ROW);
    expect(r.form.basis).toBe('100g');
    expect(r.form.values).toEqual({
      kcal: 572,
      fat_mg: 37.3,
      saturated_fat_mg: 17.3,
      carbs_mg: 49.5,
      sugars_mg: 41.2,
      protein_mg: 8.6,
      sodium_ug: 0.272,
    });
    // 21.5 g is not a whole number of grams, and is not written down as 22.
    expect(r.form.serving).toBeUndefined();
    expect(r.doubts).toEqual([]);
  });

  it('reads a unit printed before its figure (as read)', () => {
    const r = readLabel(UNITS_FIRST);
    expect(r.form.basis).toBe('100g');
    expect(r.form.energyIn).toBe('kcal');
    expect(r.form.values).toEqual({
      kcal: 492,
      fat_mg: 47,
      saturated_fat_mg: 5.3,
      carbs_mg: 11,
      sugars_mg: 5,
      fiber_mg: 3,
      protein_mg: 4.7,
      sodium_ug: 3.2,
    });
    expect(r.doubts).toEqual([]);
  });

  it('says there is nothing when the reader found no table', () => {
    expect(readLabel('NO TABLE').found).toBe(0);
    expect(readLabel('').found).toBe(0);
    expect(readLabel('Best before: 26.09.2025\nStore in a cool, dry place').found).toBe(0);
  });

  it('reads loose lines, as a small reader writes them', () => {
    const r = readLabel(
      [
        'Nutrition Information',
        'Typical values per 100g per serving 30g',
        'Energy 2567kJ/621kcal 770kJ/186kcal',
        'Fat 53.3g 16.0g',
        'of which saturates 4.3g 1.3g',
        'Carbohydrate 4.8g 1.4g',
        'of which sugars 4.8g 1.4g',
        'Protein 24.5g 7.4g',
        'Salt 0.01g <0.01g',
        '200g e',
      ].join('\n'),
    );
    expect(r.form.basis).toBe('100g');
    expect(r.form.serving).toBe(30);
    expect(r.form.values).toMatchObject({ kcal: 621, fat_mg: 53.3, protein_mg: 24.5 });
  });

  it('reads energy printed on two lines, and kJ alone when that is all there is', () => {
    const two = readLabel('| | per 100 g |\n| Energy | 1046 kJ |\n| | 250 kcal |\n| Fat | 10 g |');
    expect(two.form.values.kcal).toBe(250);
    expect(two.form.energyIn).toBe('kcal');
    const kj = readLabel('| | per 100 g |\n| Energy | 1046 kJ |');
    expect(kj.form.values.kcal).toBe(1046);
    expect(kj.form.energyIn).toBe('kJ');
  });

  it('reads a label per serving, with sodium, as an American one is', () => {
    const r = readLabel(
      [
        '| Nutrition Facts | Per serving (55 g) |',
        '|---|---|',
        '| Calories | 230 |',
        '| Total Fat | 8 g |',
        '| Saturated Fat | 1 g |',
        '| Trans Fat | 0 g |',
        '| Sodium | 160 mg |',
        '| Total Carbohydrate | 37 g |',
        '| Dietary Fiber | 4 g |',
        '| Total Sugars | 12 g |',
        '| Includes 10 g Added Sugars | 10 g |',
        '| Protein | 3 g |',
      ].join('\n'),
    );
    expect(r.form.basis).toBe('serving');
    expect(r.form.serving).toBe(55);
    expect(r.form.servingUnit).toBe('g');
    expect(r.form.sodiumAs).toBe('sodium');
    expect(r.form.values).toMatchObject({
      fat_mg: 8,
      saturated_fat_mg: 1,
      carbs_mg: 37,
      fiber_mg: 4,
      sugars_mg: 12,
      protein_mg: 3,
      sodium_ug: 160,
    });
    // "Calories" is kilocalories, printed with no unit.
    expect(r.form.values.kcal).toBe(230);
    expect(r.form.energyIn).toBe('kcal');
  });

  it.each([
    [
      'Italian',
      '| Valori medi | per 100 g |\n| Energia | 1510 kJ / 356 kcal |\n| Grassi | 1,5 g |\n| di cui acidi grassi saturi | 0,3 g |\n| Carboidrati | 71 g |\n| di cui zuccheri | 3,5 g |\n| Fibre | 3 g |\n| Proteine | 13 g |\n| Sale | 0,01 g |',
    ],
    [
      'French',
      '| Valeurs moyennes | pour 100 g |\n| Énergie | 1510 kJ / 356 kcal |\n| Matières grasses | 1,5 g |\n| dont acides gras saturés | 0,3 g |\n| Glucides | 71 g |\n| dont sucres | 3,5 g |\n| Fibres alimentaires | 3 g |\n| Protéines | 13 g |\n| Sel | 0,01 g |',
    ],
    [
      'German',
      '| Durchschnittliche Nährwerte | pro 100 g |\n| Brennwert | 1510 kJ / 356 kcal |\n| Fett | 1,5 g |\n| davon gesättigte Fettsäuren | 0,3 g |\n| Kohlenhydrate | 71 g |\n| davon Zucker | 3,5 g |\n| Ballaststoffe | 3 g |\n| Eiweiß | 13 g |\n| Salz | 0,01 g |',
    ],
    [
      'Spanish',
      '| Valores medios | por 100 g |\n| Valor energético | 1510 kJ / 356 kcal |\n| Grasas | 1,5 g |\n| de las cuales saturadas | 0,3 g |\n| Hidratos de carbono | 71 g |\n| de los cuales azúcares | 3,5 g |\n| Fibra alimentaria | 3 g |\n| Proteínas | 13 g |\n| Sal | 0,01 g |',
    ],
  ])('reads a label in %s', (_, text) => {
    const r = readLabel(text);
    expect(r.form.basis).toBe('100g');
    expect(r.form.values).toEqual({
      kcal: 356,
      fat_mg: 1.5,
      saturated_fat_mg: 0.3,
      carbs_mg: 71,
      sugars_mg: 3.5,
      fiber_mg: 3,
      protein_mg: 13,
      sodium_ug: 0.01,
    });
    expect(r.doubts).toEqual([]);
  });

  it('says what does not add up, and still shows what it read', () => {
    // A sideways photograph, as the reader invented it: fat 14 g against 170 kcal of pasta.
    const r = readLabel(
      '| | 100 g |\n| Valor energético (kcal) | 170 |\n| Gorduras totais (g) | 14 |\n| Gorduras saturadas (g) | 19 |\n| Carboidratos (g) | 28 |\n| Açúcares (g) | 30 |\n| Proteínas (g) | 6 |',
    );
    expect(r.form.values.fat_mg).toBe(14);
    expect(r.doubts).toEqual([
      'Saturates are more than the fat.',
      'Sugars are more than the carbohydrate.',
      'The energy does not match the fat, carbohydrate and protein.',
    ]);
    const split = readLabel('| | 100 g |\n| Energy | 2000 kJ / 250 kcal |');
    expect(split.doubts).toEqual(['The kJ and kcal figures do not agree with each other.']);
    const none = readLabel('| Energy | 250 kcal |\n| Fat | 10 g |');
    expect(none.form.basis).toBeUndefined();
    expect(none.doubts[0]).toMatch(/what the figures are for/);
  });
});

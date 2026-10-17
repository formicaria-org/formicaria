import { describe, expect, it } from 'vitest';
import { guessFood, ingredientLines, recipeBody } from './recipeText';

describe('a recipe note as text', () => {
  it('is written from a source and its ingredient lines read back as written', () => {
    const body = recipeBody({
      ingredients: ['Spaghetti 320 g', 'Guanciale 150 g', 'Pepe nero q.b.'],
      steps: ['Tagliate il guanciale.', 'Unite tutto.'],
      prepMinutes: 10,
      cookMinutes: 75,
      yieldText: '4 persone',
      url: 'https://www.example.org/carbonara',
      credit: 'Un sito di ricette',
      image: 'https://example.org/c.jpg',
    });
    expect(body).toContain('## Method\n\n1. Tagliate il guanciale.\n2. Unite tutto.');
    expect(body).toContain('Preparation: 10 min · Cooking: 1 h 15 min · Makes: 4 persone');
    expect(body).toContain('From [Un sito di ricette](https://www.example.org/carbonara).');
    expect(body).toContain('[The source');
    expect(ingredientLines(body)).toEqual(['Spaghetti 320 g', 'Guanciale 150 g', 'Pepe nero q.b.']);
  });
  it('reads an Ingredienti list written by hand, numbered or not, and nothing outside it', () => {
    const body =
      '# Tiramisù\n\n## Ingredienti\n\n1. 500 g di mascarpone\n* 4 uova\n\n## Preparazione\n\n- Montate i tuorli.\n';
    expect(ingredientLines(body)).toEqual(['500 g di mascarpone', '4 uova']);
  });
  it('guesses a food and its grams from the ways lines are written', () => {
    expect(guessFood('200 g di farina 00')).toEqual({ name: 'farina 00', grams: 200 });
    expect(guessFood('Guanciale 150 g')).toEqual({ name: 'Guanciale', grams: 150 });
    expect(guessFood('Riso, grammi 500.')).toEqual({ name: 'Riso', grams: 500 });
    expect(guessFood('1 kg di patate')).toEqual({ name: 'patate', grams: 1000 });
    expect(guessFood('Pepe nero q.b.')).toEqual({ name: 'Pepe nero', grams: null });
    expect(guessFood('4 uova (medie)')).toEqual({ name: 'uova', grams: null });
    expect(guessFood('Tuorli 6')).toEqual({ name: 'Tuorli', grams: null });
  });
});

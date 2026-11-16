import { describe, expect, it } from 'vitest';
import { sharedPassage } from './shared';

describe('text brought in from another app', () => {
  it('a shared page is a link, titled by what came with it', () => {
    expect(sharedPassage('Ant colonies explained https://a.org/ants', '', '2026-11-16 09:00')).toBe(
      '[Ant colonies explained](https://a.org/ants), 2026-11-16 09:00',
    );
    expect(sharedPassage('https://a.org/ants', 'Ants')).toBe('[Ants](https://a.org/ants)');
  });

  it('a passage is quoted, with its source when the address came with it', () => {
    expect(
      sharedPassage('Ants farm fungus.\nThey cut leaves for it. https://a.org/ants', 'Ants'),
    ).toBe(
      '> Ants farm fungus.\n> They cut leaves for it. https://a.org/ants\n>\n> — [Ants](https://a.org/ants)',
    );
  });

  it('a sentence with its address is a quote, not a title', () => {
    expect(sharedPassage('Ants farm fungus. https://a.org/ants', 'Ants')).toBe(
      '> Ants farm fungus. https://a.org/ants\n>\n> — [Ants](https://a.org/ants)',
    );
  });

  it('a selection from another app says it was shared, and when', () => {
    expect(sharedPassage('Ants farm fungus.', '', '2026-11-16 09:00')).toBe(
      '> Ants farm fungus.\n>\n> — shared from another app, 2026-11-16 09:00',
    );
  });
});

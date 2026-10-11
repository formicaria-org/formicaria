import { describe, expect, it } from 'vitest';
import { linkOnPaste } from './paste';

describe('pasting an address over selected words', () => {
  it('links the words to it', () => {
    expect(linkOnPaste('the paper', 'https://example.org/x')).toBe(
      '[the paper](https://example.org/x)',
    );
    expect(linkOnPaste('write to me', ' mailto:a@b.org\n')).toBe('[write to me](mailto:a@b.org)');
    expect(linkOnPaste('that note', 'note:01ARZ3NDEKTSV4RRFFQ69G5FAV')).toBe(
      '[that note](note:01ARZ3NDEKTSV4RRFFQ69G5FAV)',
    );
  });

  it('is an ordinary paste when what is pasted is not one address', () => {
    expect(linkOnPaste('words', 'just some text')).toBeNull();
    expect(linkOnPaste('words', 'see https://example.org')).toBeNull();
    expect(linkOnPaste('words', 'https://a.org https://b.org')).toBeNull();
    expect(linkOnPaste('words', '')).toBeNull();
  });

  it('is an ordinary paste with nothing selected, several lines selected, or an address selected', () => {
    expect(linkOnPaste('', 'https://example.org')).toBeNull();
    expect(linkOnPaste('   ', 'https://example.org')).toBeNull();
    expect(linkOnPaste('two\nlines', 'https://example.org')).toBeNull();
    // One address over another replaces it.
    expect(linkOnPaste('https://old.org', 'https://new.org')).toBeNull();
  });

  it('keeps brackets in the words and brackets in the address from breaking the link', () => {
    expect(linkOnPaste('see [1]', 'https://example.org')).toBe(
      '[see \\[1\\]](https://example.org)',
    );
    expect(linkOnPaste('wiki', 'https://en.wikipedia.org/wiki/A_(b)')).toBe(
      '[wiki](<https://en.wikipedia.org/wiki/A_(b)>)',
    );
  });
});

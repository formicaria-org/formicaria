import { describe, expect, it } from 'vitest';
import { SECTIONS, sectionOf } from './sections';

describe('the library sections', () => {
  it('places each item in its section, the study rooms taking the rest', () => {
    expect(sectionOf({ props: { gmail_thread: 't' } }).label).toBe('Mail');
    expect(sectionOf({ props: { log: 'music' } }).label).toBe('Music');
    expect(sectionOf({ tags: ['recipe', 'web'] }).label).toBe('Food');
    expect(sectionOf({ props: { log: 'workout' } }).label).toBe('Fitness');
    expect(sectionOf({ props: { log: 'health' } }).label).toBe('Health');
    expect(sectionOf({ tags: ['web'], props: { source_url: 'https://a.org' } }).label).toBe('Web');
    expect(sectionOf({ tags: ['lab'] }).label).toBe('Study rooms');
  });

  it('keeps every section closed to the assistant until the person opens it', () => {
    expect(SECTIONS.every((s) => s.assistant.search === false)).toBe(true);
  });

  it('opens each section as a tab by an address formicaria understands', () => {
    for (const s of SECTIONS) expect(s.address).toMatch(/^view:group:[A-Za-z]+$/);
  });
});

import { describe, expect, it } from 'vitest';
import { noteName } from './noteName';

const note = (
  over: Partial<{ title: string | null; preview: string; props: Record<string, unknown> }>,
) => ({ title: null, preview: '', props: {}, ...over });

describe('noteName', () => {
  it('is the title when there is one', () => {
    expect(noteName(note({ title: ' Reading list ', preview: '# Something else' }))).toBe(
      'Reading list',
    );
  });

  it('is an untitled note’s first line, without a heading’s marks', () => {
    expect(noteName(note({ preview: '## GAE and inner-loop adaptation' }))).toBe(
      'GAE and inner-loop adaptation',
    );
    expect(noteName(note({ preview: 'Call the lab back' }))).toBe('Call the lab back');
  });

  it('keeps the mark on a tag, which is not a heading', () => {
    expect(noteName(note({ preview: '#meta-rl reading' }))).toBe('#meta-rl reading');
  });

  it('is nothing for an untitled board, whose first line is its canvas', () => {
    expect(
      noteName(note({ preview: '{"type":"excalidraw"}', props: { view: 'board' } })),
    ).toBeNull();
  });

  it('is nothing for a note with neither', () => {
    expect(noteName(note({ title: '  ', preview: '' }))).toBeNull();
  });
});

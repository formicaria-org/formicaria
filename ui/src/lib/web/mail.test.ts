import { describe, expect, it } from 'vitest';
import { latestMessage } from './mail';

const body =
  '### Ada, 2026-11-10 09:00\n\nShall we meet Tuesday?\n<!-- gmail:1 -->\n\n### Ben, 2026-11-11 10:00\n\nTuesday works.\nAt the lab.\n<!-- gmail:2 -->\n';

describe('a passage of a mail conversation', () => {
  it('is the latest message unless something is selected', () => {
    expect(latestMessage(body)).toBe('### Ben, 2026-11-11 10:00\n\nTuesday works.\nAt the lab.');
  });
});

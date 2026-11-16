import { describe, expect, it } from 'vitest';
import { latestMessage, mailPassage } from './mail';

const body =
  '### Ada, 2026-11-10 09:00\n\nShall we meet Tuesday?\n<!-- gmail:1 -->\n\n### Ben, 2026-11-11 10:00\n\nTuesday works.\nAt the lab.\n<!-- gmail:2 -->\n';

describe('a passage of a mail conversation', () => {
  it('is the latest message unless something is selected', () => {
    expect(latestMessage(body)).toBe('### Ben, 2026-11-11 10:00\n\nTuesday works.\nAt the lab.');
  });

  it('quotes it with a link back to the conversation and its date', () => {
    const n = {
      id: '01JD0000000000000000000000',
      title: 'Lab visit',
      body,
      props: { mail_last: '2026-11-11 10:00' },
    };
    expect(mailPassage(n, 'Tuesday works.')).toBe(
      '> Tuesday works.\n>\n> — [Mail: Lab visit](note:01JD0000000000000000000000), 2026-11-11',
    );
  });
});

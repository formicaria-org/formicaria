import { describe, expect, it } from 'vitest';
import { fromNote, fromShared, piecesMarkdown } from './piece';

const ID = '01JD0000000000000000000000';

describe('one piece for whatever moves into a note', () => {
  it('a passage of a note keeps a link back to it', () => {
    const p = fromNote(
      { id: ID, title: 'Field notes', body: 'Ants at the gate.' },
      'Ants at the gate.',
      '2026-11-16',
    );
    expect(p.md).toBe(`> Ants at the gate.\n>\n> — [Field notes](note:${ID}), 2026-11-16`);
    expect(p.source).toEqual({ kind: 'study', address: `note:${ID}`, title: 'Field notes' });
  });

  it('nothing selected: a mail gives its latest message, another note a link to it', () => {
    const mail = fromNote(
      {
        id: ID,
        title: 'Lab visit',
        body: '### Ada\n\nTuesday?\n\n### Ben\n\nTuesday works.\n',
        kind_key: 'conversation',
        props: { mail_last: '2026-11-11 10:00' },
      },
      '',
      'x',
    );
    expect(mail.md).toContain('> Tuesday works.');
    expect(mail.md).toContain(`— [Mail: Lab visit](note:${ID}), 2026-11-11`);
    expect(fromNote({ id: ID, title: 'Plan', body: 'x' }, '', 'x').md).toBe(`[Plan](note:${ID})`);
  });

  it('pieces from different places read as one block', () => {
    const a = fromShared({ text: 'https://a.org/ants', subject: 'Ants' }, 'T');
    const b = fromNote({ id: ID, title: 'Plan', body: 'x' }, '', 'T');
    expect(piecesMarkdown([a, b])).toBe(`[Ants](https://a.org/ants), T\n\n[Plan](note:${ID})`);
  });
});

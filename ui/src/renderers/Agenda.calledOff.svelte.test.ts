// A meeting an email called off stays in the Agenda, struck through — never silently dropped, never
// drawn like one still on (the owner, 2026-10-15: "updates that track these changes, to avoid
// confusion").

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import Agenda from './Agenda.svelte';
import { CALLED_OFF_TAG } from '../lib/calledOff';
import type { ObjectMeta } from '../lib/types';

const card = (id: string, title: string, tags: string[]): ObjectMeta =>
  ({ id, vault: '', title, tags, due: '2099-10-16T12:00', props: {} }) as unknown as ObjectMeta;

describe('Agenda', () => {
  it('shows a called-off meeting struck through, and leaves the others alone', () => {
    render(Agenda, {
      cards: [
        card('a', 'Meeting with Marta', ['meeting', CALLED_OFF_TAG]),
        card('b', 'Meeting with Paolo', ['meeting']),
      ],
      onopen: () => {},
    } as never);
    const row = (t: string) => screen.getByText(t).closest('button')!;
    expect(row('Meeting with Marta').classList.contains('called-off')).toBe(true);
    expect(row('Meeting with Paolo').classList.contains('called-off')).toBe(false);
  });
});

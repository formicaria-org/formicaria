// **A view with nothing in it has to say so.**
//
// `EmptyState.svelte` — icon, headline, hint — existed with **zero call sites**. Every renderer
// hand-rolled a bare paragraph instead, each with its own padding and tone, and the board had no
// empty state at all: no notes means no columns, so it drew a blank flex strip and explained
// nothing. An empty screen that says nothing is indistinguishable from a broken one.

import { describe, expect, it } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import Timeline from './Timeline.svelte';
import Agenda from './Agenda.svelte';
import Discussions from './Discussions.svelte';
import Search from './Search.svelte';
import Board from './Board.svelte';

describe('every renderer explains itself when it has nothing to show', () => {
  it('timeline', () => {
    render(Timeline, { cards: [], onopen: () => {}, statuses: [] } as never);
    expect(screen.getByText('No notes yet')).toBeTruthy();
    cleanup();
  });

  it('agenda', () => {
    render(Agenda, { cards: [], onopen: () => {} } as never);
    expect(screen.getByText('Nothing on the horizon')).toBeTruthy();
    cleanup();
  });

  it('discussions', () => {
    render(Discussions, { discussions: [], shown: () => true, onopen: () => {} } as never);
    expect(screen.getByText('No discussions yet')).toBeTruthy();
    cleanup();
  });

  it('search, before anything is typed', () => {
    render(Search, { cards: [], query: '', onopen: () => {} } as never);
    expect(screen.getByText('Search your notes')).toBeTruthy();
    cleanup();
  });

  it('search, with a query that found nothing — and it names the query', () => {
    render(Search, { cards: [], query: 'lifepo4', onopen: () => {} } as never);
    expect(screen.getByText(/No matches for/)).toBeTruthy();
    expect(screen.getByText(/lifepo4/)).toBeTruthy();
    cleanup();
  });

  it('board — the one that used to render a blank strip', () => {
    render(Board, {
      board: { columns: [] },
      onmove: () => {},
      onreorder: () => {},
      onopen: () => {},
      statuses: [],
    } as never);
    expect(screen.getByText('Nothing to group yet')).toBeTruthy();
    cleanup();
  });
});

// **P1 changes nothing a person sees** (the shell, `unifying-home-research-2026-11-22.md` §11): the
// features, read now from the library's sections (`sections.json`, the shipped `.kind` files), are
// exactly the ten the app showed before, in the same order, with the same names, marks, icons and pages.
import { expect, it } from 'vitest';
import { FEATURES, featureOf } from './features';
import * as mock from './mock';
import type { Section, SectionSummary } from './types';

const BEFORE = [
  { key: 'Library', label: 'Notes', mark: '✎', icon: 'pen', url: '/f/group/Library' },
  { key: 'Tasks', label: 'Tasks', mark: '▦', icon: 'board', url: '/f/group/Tasks' },
  { key: 'agenda', label: 'Agenda', mark: '◷', icon: 'calendar', url: '/f/agenda' },
  { key: 'Food', label: 'Food journal', mark: '◍', icon: 'chart', url: '/f/group/Food' },
  { key: 'Fitness', label: 'Fitness', mark: '◆', icon: 'chart', url: '/f/group/Fitness' },
  { key: 'Health', label: 'Health', mark: '✚', icon: 'chart', url: '/f/group/Health' },
  { key: 'Music', label: 'Music', mark: '♪', icon: 'play', url: '/f/group/Music' },
  { key: 'Videos', label: 'Videos', mark: '▶', icon: 'play', url: '/f/group/Videos' },
  { key: 'discussions', label: 'Discussions', mark: '✉', icon: 'chat', url: '/f/discussions' },
  {
    key: 'collaboration',
    label: 'Collaboration',
    mark: '⇄',
    icon: 'list',
    url: '/f/collaboration',
  },
];

it('the features are the sections, unchanged from before', () => {
  expect(FEATURES).toEqual(BEFORE);
  expect(featureOf('/f/group/Food/Day')?.key).toBe('Food');
});

it('the mock answers sections and their counts', async () => {
  mock.reset();
  const list = await mock.handle<Section[]>('sections', {});
  expect(list.map((s) => s.key)).toEqual([
    'notes',
    'tasks',
    'agenda',
    'food',
    'fitness',
    'health',
    'music',
    'videos',
    'discussions',
    'collaboration',
  ]);
  const sum = await mock.handle<SectionSummary[]>('section_summary', {});
  expect(sum).toHaveLength(10);
  expect(sum.every((s) => typeof s.count === 'number')).toBe(true);
});

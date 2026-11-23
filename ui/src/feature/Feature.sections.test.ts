// **Every section, the same way** (the shell, P2a): a section's notes as a table or a gallery beside
// its own view, with a filter; and its ＋ makes what the section holds (a task on Tasks).
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';

beforeEach(() => mock.reset());
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
});

async function task(title: string, status: string) {
  await mock.handle('create_notes', { notes: [{ title, body: '', props: { status } }] });
}

it('Tasks can be seen as a table, and filtered', async () => {
  await task('Order the nest', 'doing');
  await task('Write the methods', 'todo');
  history.replaceState(null, '', '/f/group/Tasks');
  render(Feature);
  const viewAs = await screen.findByRole('group', { name: 'view as' });
  await fireEvent.click(within(viewAs).getByRole('button', { name: 'Table' }));
  const table = await screen.findByRole('table');
  await vi.waitFor(() => expect(table.textContent).toContain('Order the nest'));
  expect(table.textContent).toContain('Write the methods');
  await fireEvent.input(screen.getByRole('searchbox', { name: 'filter Tasks' }), {
    target: { value: 'nest' },
  });
  await vi.waitFor(() =>
    expect(screen.getByRole('table').textContent).not.toContain('Write the methods'),
  );
  // Back to the section's own view.
  await fireEvent.click(within(viewAs).getByRole('button', { name: 'As usual' }));
  expect(screen.queryByRole('table')).toBeNull();
});

it('Notes can be seen as a gallery', async () => {
  await mock.handle('create_notes', { notes: [{ title: 'Field notes', body: 'x', props: {} }] });
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  const viewAs = await screen.findByRole('group', { name: 'view as' });
  await fireEvent.click(within(viewAs).getByRole('button', { name: 'Gallery' }));
  const g = await screen.findByRole('list', { name: 'gallery' });
  await vi.waitFor(() => expect(g.textContent).toContain('Field notes'));
});

it('＋ New on Tasks makes a task', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(Feature);
  await fireEvent.click(await screen.findByRole('button', { name: /New note/ }));
  await vi.waitFor(() => expect(opened).toHaveBeenCalled());
  const id = String(opened.mock.calls[0][0]).split('/').pop()!;
  const made = (await mock.handle('get', { id })) as {
    props?: Record<string, unknown>;
    status?: string;
  };
  expect(made.status ?? made.props?.status).toBe('todo');
});

it('Web has no view of its own: its page opens on its table, and its ＋ opens the one box', async () => {
  history.replaceState(null, '', '/f/group/Web');
  const asked: string[] = [];
  addEventListener('keydown', (e) => e.ctrlKey && asked.push(e.key), { once: true });
  render(Feature, { path: '/f/group/Web', go: () => {} });
  const as = await screen.findByRole('group', { name: 'view as' });
  expect(within(as).queryByRole('button', { name: 'As usual' })).toBeNull();
  expect(within(as).getByRole('button', { name: 'Table' }).getAttribute('aria-pressed')).toBe(
    'true',
  );
  await fireEvent.click(screen.getByRole('button', { name: '＋ New note' }));
  expect(asked).toEqual(['k']);
});

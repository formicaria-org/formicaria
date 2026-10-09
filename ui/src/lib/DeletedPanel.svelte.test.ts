import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, expect, test, vi } from 'vitest';
import DeletedPanel from './DeletedPanel.svelte';
import { capture, deleteNote, getNote } from './ipc';
import { reset } from './mock';

afterEach(() => reset());

test('Recently deleted lists a deleted note and brings it back', async () => {
  const n = await capture('# Field trip\n\nmeet at the gate', '');
  await deleteNote(n.id);
  const restored = vi.fn();
  render(DeletedPanel, { onclose: () => {}, onrestored: restored });
  expect(await screen.findByText('Field trip')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Bring back' }));
  await waitFor(() => expect(restored).toHaveBeenCalled());
  expect((await getNote(n.id))?.body).toContain('meet at the gate');
  expect(screen.queryByText('Field trip')).toBeNull();
});

test('an empty list says so', async () => {
  render(DeletedPanel, { onclose: () => {}, onrestored: () => {} });
  expect(await screen.findByText(/Nothing was deleted/)).toBeTruthy();
});

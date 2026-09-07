/// **Three questions on top, the whole file underneath.**
///
/// The thing worth pinning is not that a colour picker works — it is the boundary. The form writes
/// a closed list of tokens; when the file says more than the form can, the form must go quiet and
/// refuse to rewrite it rather than flattening someone's CSS into three answers. That is the rule
/// `views::save_view` already applies to a filter richer than one tag, and it is the difference
/// between a form over a file and a form that eats files.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';

const { listThemes, readTheme, saveTheme, deleteTheme } = vi.hoisted(() => ({
  listThemes: vi.fn(),
  readTheme: vi.fn(),
  saveTheme: vi.fn(),
  deleteTheme: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  listThemes,
  readTheme,
  saveTheme,
  deleteTheme,
}));

import Appearance from './Appearance.svelte';

const MINE = { name: 'my-appearance', vault: 'personal', bytes: 64 };

beforeEach(() => {
  vi.clearAllMocks();
  listThemes.mockResolvedValue([]);
  readTheme.mockResolvedValue('');
  saveTheme.mockResolvedValue([MINE]);
  deleteTheme.mockResolvedValue([]);
  document.getElementById('fm-theme')?.remove();
});
afterEach(() => vi.unstubAllGlobals());

test('picking a colour makes a theme and switches to it', async () => {
  const onselect = vi.fn();
  render(Appearance, { selected: null, onselect });

  await fireEvent.click(await screen.findByLabelText('accent #2563eb'));

  // One colour answers the four tokens that have to agree, and the file is CSS, not a blob.
  await waitFor(() => expect(saveTheme).toHaveBeenCalled());
  const css = saveTheme.mock.calls[0][1] as string;
  expect(css).toContain('--accent: #2563eb;');
  expect(css).toContain('--link: #2563eb;');
  // A first-timer who picks a colour must *see* it — creating the file without wearing it would
  // look like the click did nothing.
  await waitFor(() =>
    expect(onselect).toHaveBeenCalledWith({ vault: 'personal', name: 'my-appearance' }),
  );
});

test('a theme with more in it than the form can say switches the form off', async () => {
  listThemes.mockResolvedValue([{ name: 'writing-desk', vault: 'personal', bytes: 400 }]);
  // A selector: nothing three questions can round-trip.
  readTheme.mockResolvedValue(':root { --bg: #f4f1ea; }\n.pane-head { display: none; }');
  render(Appearance, { selected: { vault: 'personal', name: 'writing-desk' }, onselect: vi.fn() });

  await screen.findByText(/more in it than these controls can describe/i);
  const swatch = await screen.findByLabelText('accent #2563eb');
  expect((swatch as HTMLButtonElement).disabled).toBe(true);

  // And the refusal must be real, not just visual.
  await fireEvent.click(swatch);
  expect(saveTheme).not.toHaveBeenCalled();
});

test('editing previews live, and cancelling puts back exactly what was there', async () => {
  listThemes.mockResolvedValue([{ name: 'writing-desk', vault: 'personal', bytes: 40 }]);
  readTheme.mockResolvedValue(':root { --bg: #f4f1ea; }');
  render(Appearance, { selected: { vault: 'personal', name: 'writing-desk' }, onselect: vi.fn() });

  await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
  const box = await screen.findByLabelText(/applied as you type/i);
  await fireEvent.input(box, { target: { value: ':root { --bg: #000000; }' } });
  // Live: the page is already wearing the draft, before anything is saved.
  expect(document.getElementById('fm-theme')?.textContent).toBe(':root { --bg: #000000; }');
  expect(saveTheme).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() =>
    expect(document.getElementById('fm-theme')?.textContent).toBe(':root { --bg: #f4f1ea; }'),
  );
  expect(saveTheme).not.toHaveBeenCalled();
});

test('keeping writes the file and wears it', async () => {
  listThemes.mockResolvedValue([{ name: 'writing-desk', vault: 'personal', bytes: 40 }]);
  readTheme.mockResolvedValue(':root { --bg: #f4f1ea; }');
  const onselect = vi.fn();
  render(Appearance, { selected: { vault: 'personal', name: 'writing-desk' }, onselect });

  await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
  const box = await screen.findByLabelText(/applied as you type/i);
  await fireEvent.input(box, { target: { value: ':root { --bg: #111111; }' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Keep' }));

  await waitFor(() =>
    expect(saveTheme).toHaveBeenCalledWith('writing-desk', ':root { --bg: #111111; }', 'personal'),
  );
  expect(onselect).toHaveBeenCalledWith({ vault: 'personal', name: 'writing-desk' });
});

test('a theme that cannot be applied is listed with its reason and cannot be chosen', async () => {
  listThemes.mockResolvedValue([
    {
      name: 'broken',
      vault: 'personal',
      bytes: 3,
      error: 'this file is not text, so it cannot be a theme',
    },
  ]);
  render(Appearance, { selected: null, onselect: vi.fn() });

  await screen.findByText(/not text, so it cannot be a theme/i);
  // Listed, never silently dropped — and not selectable, because it would do nothing.
  const radios = screen.getAllByRole('radio') as HTMLInputElement[];
  expect(radios.some((r) => r.disabled)).toBe(true);
});

test('deleting the theme in use takes it off first', async () => {
  listThemes.mockResolvedValue([{ name: 'writing-desk', vault: 'personal', bytes: 40 }]);
  const onselect = vi.fn();
  render(Appearance, { selected: { vault: 'personal', name: 'writing-desk' }, onselect });

  await fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));
  // Order matters: still wearing a file that no longer exists is a blank-screen bug.
  await waitFor(() => expect(onselect).toHaveBeenCalledWith(null));
  expect(deleteTheme).toHaveBeenCalledWith('writing-desk', 'personal');
});

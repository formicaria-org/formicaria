// **What a page tells the usage diary** (`lib/diary.ts`, `fm_app::diary`, 2026-11-20): its kind and
// timing, errors cleaned of what could name a person's things, in batches; and the switch in Settings.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import * as diary from './diary';
import * as mock from './mock';
import UsageDiarySection from './UsageDiarySection.svelte';

beforeEach(() => mock.reset());

it('an error loses anything quoted, addresses, paths, emails and long numbers', () => {
  const m = diary.clean(
    `could not open "Diary of May" at https://bank.example/a in /home/x/notes for me@example.com id 12345678`,
  );
  for (const gone of ['Diary of May', 'bank.example', '/home', 'me@example.com', '12345678'])
    expect(m).not.toContain(gone);
  expect(m.startsWith('could not open')).toBe(true);
  expect(diary.clean('word '.repeat(100), 30).length).toBeLessThanOrEqual(30);
});

it('pages and errors go in one batch, sent as the page goes', async () => {
  diary.pageOpened('Food');
  diary.error(`TypeError: "Mum's birthday" is undefined`);
  expect(diary.waiting()).toHaveLength(2);
  diary.send();
  await vi.waitFor(() => expect(mock.diaryLines()).toHaveLength(2));
  const [page, err] = mock.diaryLines() as Record<string, unknown>[];
  expect(page).toMatchObject({ kind: 'page', page: 'Food' });
  expect(String(err.message)).not.toContain('Mum');
  expect(diary.waiting()).toHaveLength(0);
});

it('Settings: the switch, what is kept, and Delete it', async () => {
  diary.pageOpened('Food');
  diary.send();
  render(UsageDiarySection);
  const sw = (await screen.findByRole('switch', { name: 'Usage diary' })) as HTMLInputElement;
  expect(sw.checked).toBe(true);
  expect(screen.getByText(/never sent anywhere else/)).toBeTruthy();
  await vi.waitFor(() => expect(screen.getByText(/since 2026-11-20/)).toBeTruthy());
  await fireEvent.click(screen.getByRole('button', { name: 'Delete it' }));
  await vi.waitFor(() => expect(screen.getByText('Nothing kept yet.')).toBeTruthy());
  await fireEvent.click(sw);
  await vi.waitFor(() => expect(sw.checked).toBe(false));
  diary.pageOpened('Food');
  diary.send();
  await new Promise((r) => setTimeout(r, 0));
  expect(mock.diaryLines()).toHaveLength(0);
});

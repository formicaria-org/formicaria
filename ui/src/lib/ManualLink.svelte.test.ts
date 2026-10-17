// "How this works" opens the manual at the section about the thing beside it (2026-10-18).
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, expect, test } from 'vitest';
import ManualLink from './ManualLink.svelte';
import ManualPanel from './ManualPanel.svelte';
import { closeManual } from './manual.svelte';
import { asDesktop, asPhone } from './harness';

afterEach(() => {
  closeManual();
  asDesktop();
});

test('a link opens the reader at its page and section', async () => {
  render(ManualPanel);
  render(ManualLink, { page: 'user/appearance.html', section: 'starting-from-a-suggestion' });
  await fireEvent.click(screen.getByRole('button', { name: 'How this works' }));
  const frame = (await screen.findByTitle('The manual')) as HTMLIFrameElement;
  expect(frame.getAttribute('src')).toBe('/manual/user/appearance.html#starting-from-a-suggestion');
});

test('on the phone the same link reads the copy the app carries', async () => {
  asPhone();
  render(ManualPanel);
  render(ManualLink, { page: 'user/backup.html', section: 'setting-up-once' });
  await fireEvent.click(screen.getByRole('button', { name: 'How this works' }));
  const frame = (await screen.findByTitle('The manual')) as HTMLIFrameElement;
  expect(frame.getAttribute('src')).toBe(
    'http://fmmanual.localhost/user/backup.html#setting-up-once',
  );
});

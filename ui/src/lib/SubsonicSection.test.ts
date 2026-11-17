// **Your music in other players** (`SubsonicSection.svelte`): off until turned on; then the address,
// the user and a password to put in a Subsonic player, which can be renewed.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, expect, it } from 'vitest';
import SubsonicSection from './SubsonicSection.svelte';
import * as mock from './mock';

beforeEach(() => mock.reset());

it('is turned on, and shows what a player needs', async () => {
  render(SubsonicSection);
  await fireEvent.click(await screen.findByRole('button', { name: 'Turn it on' }));
  expect(await screen.findByText('formicaria')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Show' }));
  const pw = ((await mock.handle('subsonic', {})) as { password: string }).password;
  expect(screen.getByText(pw)).toBeTruthy();
  expect(screen.getByText(/reaches this computer only while/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'New password' }));
  await screen.findByRole('button', { name: 'Turn it off' });
});

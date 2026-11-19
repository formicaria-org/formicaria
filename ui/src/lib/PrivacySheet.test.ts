// **Privacy** (`decisions.md` 2026-11-17): each protection a switch, kept per device by the server,
// handed to the phone's page views; a site's protections off; the fire button asks first; the web
// field searches the engine chosen.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import PrivacySheet from './PrivacySheet.svelte';
import NewTab from '../feature/NewTab.svelte';
import * as mock from './mock';
import { loadPrivacy } from './privacy.svelte';

const g = globalThis as Record<string, unknown>;
let told: string[] = [];
beforeEach(async () => {
  mock.reset();
  told = [];
  g.__fmPage = { privacy: (j: string) => told.push(j) };
  await loadPrivacy();
});
afterEach(() => {
  delete g.__fmPage;
});

it('shows the switches as this device has them, and a change reaches the server and the shell', async () => {
  render(PrivacySheet);
  const cookies = (await screen.findByRole('switch', {
    name: 'Refuse cookie pop-ups',
  })) as HTMLInputElement;
  expect(cookies.checked).toBe(true);
  expect((screen.getByRole('switch', { name: 'Block ads' }) as HTMLInputElement).checked).toBe(
    false,
  );
  await fireEvent.click(cookies);
  await waitFor(async () =>
    expect(((await mock.handle('privacy', {})) as { cookies: boolean }).cookies).toBe(false),
  );
  expect(JSON.parse(told.at(-1)!).cookies).toBe(false);
});

it('turns protections off for the page’s site, and back on', async () => {
  render(PrivacySheet, {
    site: 'https://www.news.example/story?id=1',
    counts: { blocked: 3, banners: 1 },
  });
  expect(await screen.findByText(/3 blocked · 1 pop-ups refused/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Protections off for this site' }));
  await screen.findByRole('button', { name: 'Protect this site again' });
  expect(((await mock.handle('privacy', {})) as { sites_off: string[] }).sites_off).toEqual([
    'news.example',
  ]);
  expect(screen.getByRole('button', { name: 'protect news.example again' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Protect this site again' }));
  await waitFor(async () =>
    expect(((await mock.handle('privacy', {})) as { sites_off: string[] }).sites_off).toEqual([]),
  );
});

it('the fire button asks before forgetting', async () => {
  render(PrivacySheet);
  await fireEvent.click(await screen.findByRole('button', { name: /Forget web data/ }));
  expect(screen.getByText(/Your notes and the files you kept stay/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Keep them' }));
  expect(((await mock.handle('privacy', {})) as { fired: number }).fired).toBe(0);
  await fireEvent.click(screen.getByRole('button', { name: /Forget web data/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Forget them' }));
  await waitFor(async () =>
    expect(((await mock.handle('privacy', {})) as { fired: number }).fired).toBe(1),
  );
  expect(JSON.parse(told.at(-1)!).fired).toBe(1);
});

it('other apps’ trackers: what each app tried, and one let through', async () => {
  const allowed: string[] = [];
  g.__fmPage = { privacy: () => {}, vpn: () => true };
  g.__fmVpn = {
    state: () =>
      JSON.stringify({
        on: true,
        apps: [
          {
            pkg: 'com.example.weather',
            label: 'Weather',
            n: 41,
            hosts: [{ host: 'ads.track.example', n: 41 }],
          },
        ],
        allowed,
      }),
    allow: (pkg: string, through: boolean) => (through ? allowed.push(pkg) : null),
  };
  try {
    render(PrivacySheet, { vpn: true });
    await fireEvent.click(
      await screen.findByRole('switch', { name: 'Block trackers in other apps' }),
    );
    expect(await screen.findByText('Weather — 41 refused')).toBeTruthy();
    expect(screen.getByText('ads.track.example')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Let it through' }));
    expect(allowed).toEqual(['com.example.weather']);
    // Android's permission refused: the switch goes off again, and says why.
    window.dispatchEvent(new CustomEvent('fm-vpn', { detail: { refused: true } }));
    expect(await screen.findByText(/permission was not given/)).toBeTruthy();
    await waitFor(async () =>
      expect(((await mock.handle('privacy', {})) as { app_trackers: boolean }).app_trackers).toBe(
        false,
      ),
    );
  } finally {
    delete g.__fmVpn;
  }
});

it('other apps’ trackers are offered only where the build can block them', async () => {
  render(PrivacySheet);
  const sw = (await screen.findByRole('switch', {
    name: 'Block trackers in other apps',
  })) as HTMLInputElement;
  expect(sw.disabled).toBe(true);
  expect(screen.getByText(/Not in this build of formicaria/)).toBeTruthy();
});

it('the web field searches the engine chosen', async () => {
  render(PrivacySheet);
  const pick = (await screen.findByRole('combobox', { name: /Search with/ })) as HTMLSelectElement;
  await fireEvent.change(pick, { target: { value: 'mojeek' } });
  await waitFor(async () =>
    expect(((await mock.handle('privacy', {})) as { search: string }).search).toBe('mojeek'),
  );
  let went = '';
  render(NewTab, { go: (u: string) => (went = u) });
  const field = screen.getByPlaceholderText('A web address, or words to search with Mojeek');
  await fireEvent.input(field, { target: { value: 'leafcutter ants' } });
  await fireEvent.click(
    screen.getByRole('button', { name: 'Search the web for “leafcutter ants”' }),
  );
  await waitFor(() => expect(went).toContain('mojeek.com/search?q=leafcutter%20ants'));
});

// **Encrypted lookups, on by default** (the owner, 2026-11-19), and the resolver chosen.
it('lookups go encrypted to Quad9 at first, and another resolver can be chosen', async () => {
  render(PrivacySheet);
  const pick = (await screen.findByRole('combobox', {
    name: 'Encrypted lookups',
  })) as HTMLSelectElement;
  expect(pick.value).toBe('quad9');
  expect([...pick.options].map((o) => o.value)).toEqual(['quad9', 'mullvad', 'network']);
  await fireEvent.change(pick, { target: { value: 'mullvad' } });
  await waitFor(async () =>
    expect(((await mock.handle('privacy', {})) as { dns: string }).dns).toBe('mullvad'),
  );
  expect(JSON.parse(told.at(-1)!).dns).toBe('mullvad');
});

// **What the app block cannot see, said** (`privacy-plan-2026-11-19.md` 2.6), and the switch that
// stops apps going around it (4.3), offered once the block is on.
it('the app block warns when Private DNS hides lookups, and offers to stop apps going around it', async () => {
  g.__fmPage = { privacy: () => {}, vpn: () => true };
  g.__fmVpn = {
    state: () =>
      JSON.stringify({ on: true, apps: [], allowed: [], ready: true, private_dns: 'dns.example' }),
    allow: () => {},
  };
  try {
    render(PrivacySheet, { vpn: true });
    expect(screen.queryByRole('switch', { name: 'Stop apps going around the block' })).toBeNull();
    await fireEvent.click(
      await screen.findByRole('switch', { name: 'Block trackers in other apps' }),
    );
    expect((await screen.findByRole('alert')).textContent).toContain('dns.example');
    const around = (await screen.findByRole('switch', {
      name: 'Stop apps going around the block',
    })) as HTMLInputElement;
    expect(around.checked).toBe(false);
    await fireEvent.click(around);
    await waitFor(async () =>
      expect(((await mock.handle('privacy', {})) as { around: boolean }).around).toBe(true),
    );
  } finally {
    delete g.__fmVpn;
  }
});

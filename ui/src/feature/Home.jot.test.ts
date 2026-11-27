// **The phone's jot box says what Enter does, and the web is one tap away** (the owner, 2026-11-25:
// *"if I type something and enter it actually creates a note, there is no web there"*). Words are
// jotted into today's entry by Enter, with a search for them beside it; an address is opened by
// Enter, with jotting it beside it.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import HomeView from './HomeView.svelte';
import * as mock from '../lib/mock';
import { resetShownVaults } from '../lib/shownVaults.svelte';

beforeEach(() => {
  mock.reset();
  resetShownVaults();
  // A phone: Home measures itself narrower than 700 px.
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.classList?.contains('home') ? 400 : 0;
  });
});
afterEach(() => vi.restoreAllMocks());

const box = () => screen.findByRole('textbox', { name: 'jot into today or search the web' });
const choices = async () =>
  within(await screen.findByRole('group', { name: 'what to do with it' }))
    .getAllByRole('button')
    .map((b) => b.textContent?.trim());

it('words: Enter jots them, and a search for them is offered beside', async () => {
  const go = vi.fn();
  render(HomeView, { go, onopen: () => {} });
  const input = await box();
  expect(screen.queryByRole('group', { name: 'what to do with it' })).toBeNull();
  await fireEvent.input(input, { target: { value: 'ant colonies' } });
  expect(await choices()).toEqual(['Add to today↵', 'Search the web for “ant colonies”']);
  await fireEvent.submit(input.closest('form')!);
  await screen.findByText('Added to today’s journal.');
  expect(go).not.toHaveBeenCalled();
  // The other choice, pressed: the web, not a note.
  await fireEvent.input(input, { target: { value: 'ant colonies' } });
  await fireEvent.click(screen.getByRole('button', { name: /^Search the web/ }));
  expect(go).toHaveBeenCalledWith(expect.stringContaining('ant%20colonies'));
});

it('an address: Enter opens it, and jotting it is offered beside', async () => {
  const go = vi.fn();
  render(HomeView, { go, onopen: () => {} });
  const input = await box();
  await fireEvent.input(input, { target: { value: 'scholar.google.com' } });
  expect(await choices()).toEqual(['Go to scholar.google.com↵', 'Add to today']);
  await fireEvent.submit(input.closest('form')!);
  expect(go).toHaveBeenCalledWith('https://scholar.google.com');
});

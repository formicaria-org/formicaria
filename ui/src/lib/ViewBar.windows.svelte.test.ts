// **On a phone, the open windows are one counted button with the list behind it.**
//
// The owner's phone, 2026-09-11: three open windows were three tabs and a close button across the whole
// top row. A narrow screen now shows the window you are in and one square carrying the number of open
// windows; pressing it lists them, and a row switches to its window or closes it.
//
// jsdom applies no CSS, so *which* presentation shows at a given width is not tested here — that is
// checked by screenshot. What is pinned is that the counted button and its list do what they say.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ViewBar from './ViewBar.svelte';
import { newPane } from './panes';

function bar() {
  const panes = [newPane('timeline'), newPane('agenda'), newPane('activity')];
  const onselect = vi.fn();
  const onclose = vi.fn();
  render(ViewBar, {
    props: { panes, active: 1, feed: undefined, onselect, onchange: () => {}, onclose },
  });
  return { panes, onselect, onclose };
}

describe('the counted button for open windows', () => {
  it('carries the number of open windows', () => {
    bar();
    const button = screen.getByRole('button', { name: 'open windows: 3' });
    expect(button.textContent?.trim()).toBe('3');
    expect(button.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('dialog', { name: 'open windows' })).toBeNull();
  });

  it('lists every window, marks the one you are in, and switches to the one pressed', async () => {
    const { onselect } = bar();
    await fireEvent.click(screen.getByRole('button', { name: 'open windows: 3' }));

    const list = screen.getByRole('dialog', { name: 'open windows' });
    const rows = within(list).getAllByRole('listitem');
    expect(rows).toHaveLength(3);
    expect(
      within(rows[1]).getByRole('button', { name: 'Agenda' }).getAttribute('aria-current'),
    ).toBe('true');
    expect(
      within(rows[0]).getByRole('button', { name: 'Timeline' }).getAttribute('aria-current'),
    ).toBeNull();

    await fireEvent.click(within(rows[2]).getByRole('button', { name: 'Activity' }));
    expect(onselect).toHaveBeenCalledWith(2);
    // Choosing is the end of choosing: the list goes away.
    expect(screen.queryByRole('dialog', { name: 'open windows' })).toBeNull();
  });

  it('closes a window from its own row, and Escape puts the list away', async () => {
    const { panes, onclose } = bar();
    await fireEvent.click(screen.getByRole('button', { name: 'open windows: 3' }));

    await fireEvent.click(screen.getByRole('button', { name: 'close Timeline' }));
    expect(onclose).toHaveBeenCalledWith(panes[0].id);

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'open windows' })).toBeNull();
  });
});

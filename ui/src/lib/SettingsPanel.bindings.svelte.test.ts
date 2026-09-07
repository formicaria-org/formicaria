// **A shortcut belongs next to the thing it does.**
//
// Per-command rebinding has shipped for a while and was invisible unless you scrolled to the
// Keyboard section at the bottom of this panel — so the accelerators existed and nobody could see
// them, which is the opposite of Nielsen's seventh heuristic. The chip in the action list is the
// standard menu pattern, and it also makes *unbound* visible: `newView` ships with no default key,
// a fact that was previously learnable only by reading a source comment.

import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
});

function panel(commands: unknown[]) {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'dark',
    ontheme: () => {},
    commands,
  } as never);
}

describe('the action list shows what each action is bound to', () => {
  it('a bound command carries its shortcut', async () => {
    panel([{ group: 'App', label: 'Settings', run: () => {}, command: 'palette' }]);
    const btn = await screen.findByRole('button', { name: /Settings/ });
    // `palette` defaults to Ctrl+K; the exact rendering is `keys.describe`'s business, so this
    // asserts the key is named rather than a particular spelling of the modifier.
    expect(btn.textContent).toMatch(/K/);
  });

  it('a command with no key says so instead of showing nothing', async () => {
    // `newView` ships unbound. A blank space would read as "there is no shortcut for this",
    // which is a different claim from "you have not set one yet".
    panel([
      {
        group: 'This view',
        label: 'Keep this arrangement as a view',
        run: () => {},
        command: 'newView',
      },
    ]);
    const btn = await screen.findByRole('button', { name: /Keep this arrangement/ });
    expect(btn.textContent).toMatch(/not set/i);
  });

  it('an action that is not a bindable command carries no chip at all', async () => {
    panel([{ group: 'Create', label: 'New vault', run: () => {} }]);
    const btn = await screen.findByRole('button', { name: 'New vault' });
    expect(btn.textContent?.trim()).toBe('New vault');
  });
});

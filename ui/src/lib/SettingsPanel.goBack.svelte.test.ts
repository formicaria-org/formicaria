// **Going back has to be reachable, and it has to be hard to do by accident.**
//
// An update replaces the program in place and keeps the one it replaced. The launcher restores that
// copy automatically, but only for a version that cannot start at all — a version that installs,
// runs, stays up and is simply *worse* has no automatic signal, so a person has to be able to say
// so. Without this control the answer would be "delete the `program` folder and start it again",
// which is not something to tell a user who has no terminal.
//
// The other half is the arming. This replaces the running program and cannot be undone from the
// panel, so it is two steps like "Remove the model…" and "forget this vault": the first press names
// what will happen, the second does it.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

const { updateStatus, updateRollback, agentStatus } = vi.hoisted(() => ({
  updateStatus: vi.fn(),
  updateRollback: vi.fn(),
  agentStatus: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  updateStatus,
  updateRollback,
  agentStatus,
}));

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
  } as never);
}

const status = (over: Record<string, unknown> = {}) => ({
  can_check: true,
  can_install: true,
  why: '',
  current: 'v0.6.0',
  available: null,
  previous: 'v0.5.1',
  can_go_back: true,
  checking: false,
  check: true,
  last_check: 0,
  error: null,
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  updateStatus.mockResolvedValue(status());
  updateRollback.mockResolvedValue({ restarting: true });
  // The assistant section shares this panel; keep it out of the way.
  agentStatus.mockRejectedValue(new Error('no assistant here'));
});

describe('SettingsPanel — going back to the earlier version', () => {
  it('offers the version it would go back to, by name', async () => {
    panel();
    expect(await screen.findByText(/Go back to v0\.5\.1/)).toBeTruthy();
  });

  /// The whole point of arming: one press must not replace the program.
  it('does not go back on the first press — it says what will happen', async () => {
    panel();
    await fireEvent.click(await screen.findByText(/Go back to v0\.5\.1/));

    expect(updateRollback).not.toHaveBeenCalled();
    expect(await screen.findByText(/go back to v0\.5\.1\?/)).toBeTruthy();
    // And it names the consequence, including the one people worry about.
    expect(screen.getByText(/Your notes are not touched/)).toBeTruthy();
  });

  it('goes back on the second press', async () => {
    panel();
    await fireEvent.click(await screen.findByText(/Go back to v0\.5\.1/));
    await fireEvent.click(await screen.findByText(/Yes, go back/));
    expect(updateRollback).toHaveBeenCalledOnce();
  });

  it('can be backed out of without doing anything', async () => {
    panel();
    await fireEvent.click(await screen.findByText(/Go back to v0\.5\.1/));
    await fireEvent.click(await screen.findByText(/^Cancel$/));

    expect(updateRollback).not.toHaveBeenCalled();
    expect(await screen.findByText(/Go back to v0\.5\.1/)).toBeTruthy();
  });

  /// A fresh download has nothing to go back to. Showing a disabled control would be offering
  /// something that cannot work; the row is simply absent.
  it('shows nothing at all when there is no earlier version', async () => {
    updateStatus.mockResolvedValue(status({ previous: null, can_go_back: false }));
    panel();
    await screen.findByText(/This machine/);
    expect(screen.queryByText(/Go back to/)).toBeNull();
  });

  /// **Being able to see is not being able to act.** A folder the system protects can still be told
  /// what it is running; it cannot replace it. Conflating the two is how a panel comes to offer a
  /// control that reports success and does nothing.
  it('shows nothing when this copy cannot install anything', async () => {
    updateStatus.mockResolvedValue(status({ can_install: false, why: 'this folder is read-only' }));
    panel();
    await screen.findByText(/This machine/);
    expect(screen.queryByText(/Go back to/)).toBeNull();
  });

  /// A build without the updater answers nothing; the panel must still render.
  it('survives a build that has no updater at all', async () => {
    updateStatus.mockRejectedValue(new Error('unknown command: update_status'));
    panel();
    await screen.findByText(/This machine/);
    expect(screen.queryByText(/Go back to/)).toBeNull();
  });

  it('prints the reason verbatim when going back is refused', async () => {
    updateRollback.mockRejectedValue(
      new Error('the earlier version will not run on this computer'),
    );
    panel();
    await fireEvent.click(await screen.findByText(/Go back to v0\.5\.1/));
    await fireEvent.click(await screen.findByText(/Yes, go back/));
    expect(await screen.findByText(/will not run on this computer/)).toBeTruthy();
  });
});

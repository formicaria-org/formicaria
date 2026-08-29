// **The one question git asks that the app should ask first.**
//
// Git refuses to commit without a committer, and the app covers for it with a placeholder — which
// is permanent in a way a setting is not, because it is stamped into every commit and history does
// not get corrected later. So it is asked once, at the start, and the three things that decide
// whether it is *right* are all about restraint (`outstanding.md` §2.6):
//
//   - skipping must cost nothing and must not save anything;
//   - the name must survive a bad remote, because those fail for unrelated reasons;
//   - the identity is saved first and on its own, since `set_git_remote` refuses an empty URL.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { setIdentity, setGitRemote } = vi.hoisted(() => ({
  setIdentity: vi.fn(),
  setGitRemote: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  setIdentity,
  setGitRemote,
}));

import Welcome from './Welcome.svelte';

const type = async (label: RegExp, value: string) =>
  fireEvent.input(screen.getByLabelText(label), { target: { value } });

beforeEach(() => {
  vi.clearAllMocks();
  setIdentity.mockResolvedValue(undefined);
  setGitRemote.mockResolvedValue(undefined);
});

describe('the welcome screen', () => {
  it('saves a name and email, and asks for no remote', async () => {
    const ondone = vi.fn();
    render(Welcome, { vault: 'personal', ondone });

    await type(/Your name/i, 'Ada Lovelace');
    await type(/Your email/i, 'ada@example.org');
    await fireEvent.click(screen.getByRole('button', { name: /Start writing/i }));

    expect(setIdentity).toHaveBeenCalledWith('Ada Lovelace', 'ada@example.org', 'personal');
    // An empty URL is refused by `set_git_remote`, so an optional field left empty must not reach it.
    expect(setGitRemote).not.toHaveBeenCalled();
    expect(ondone).toHaveBeenCalled();
  });

  it('keeps the name when the remote is the thing that fails', async () => {
    setGitRemote.mockRejectedValue(new Error("'nonsense' is not a repository URL"));
    const ondone = vi.fn();
    render(Welcome, { ondone });

    await type(/Your name/i, 'Ada Lovelace');
    await type(/Your email/i, 'ada@example.org');
    await type(/backup repository/i, 'nonsense');
    await fireEvent.click(screen.getByRole('button', { name: /Start writing/i }));

    // The identity call already returned, so it is stored — and the message has to say so, or the
    // user retypes a name that is already saved, or walks away thinking nothing was.
    expect(setIdentity).toHaveBeenCalled();
    expect(await screen.findByText(/Your name was saved/i)).toBeTruthy();
    expect(await screen.findByText(/not a repository URL/i)).toBeTruthy();
    expect(ondone).not.toHaveBeenCalled();
  });

  it('lets you past without saving anything at all', async () => {
    const ondone = vi.fn();
    render(Welcome, { ondone });

    await fireEvent.click(screen.getByRole('button', { name: /Skip for now/i }));

    expect(ondone).toHaveBeenCalled();
    // A notebook that demands a name before it will hold a note has misunderstood what it is: the
    // placeholder identity keeps history working, and BackupPanel asks again when it matters.
    expect(setIdentity).not.toHaveBeenCalled();
    expect(setGitRemote).not.toHaveBeenCalled();
  });

  it('will not save half an identity', async () => {
    render(Welcome, { ondone: () => {} });
    await type(/Your name/i, 'Ada Lovelace');
    // Git needs both; offering an enabled button that the backend refuses is the control this
    // codebase keeps ruling against.
    const start = screen.getByRole('button', { name: /Start writing/i }) as HTMLButtonElement;
    expect(start.disabled).toBe(true);
  });
});

// The in-app raw editor for notes that won't render (a conflict, or corrupt YAML). Since the UI is
// the only way in — and the phone has no OS editor — this flow IS the fix, so its wiring is worth a
// component test: Fix here → read the raw text → live "has markers?" feedback → Save → resolveSkipped.
// The ipc calls are stubbed (there is no unreadable note in the mock backend); the backend behaviour
// itself is covered in fm-app/tests/skipped.rs.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

const { readSkipped, resolveSkipped, openSkipped } = vi.hoisted(() => ({
  readSkipped: vi.fn(),
  resolveSkipped: vi.fn(),
  openSkipped: vi.fn(),
}));
vi.mock('./ipc', () => ({ readSkipped, resolveSkipped, openSkipped }));

import SkippedPanel from './SkippedPanel.svelte';

const CONFLICTED =
  '---\nid: X\ntype: note\ncreated: c\nupdated: u\n<<<<<<< ours\nstatus: doing\n=======\nstatus: done\n>>>>>>> theirs\n---\nbody';
const RESOLVED = '---\nid: X\ntype: note\ncreated: c\nupdated: u\nstatus: done\n---\nbody';

describe('SkippedPanel in-app editor', () => {
  it('reads the raw text, flags markers live, and saves the resolution', async () => {
    readSkipped.mockResolvedValue({ text: CONFLICTED });
    resolveSkipped.mockResolvedValue({ parses: true });

    render(SkippedPanel, {
      skipped: [{ vault: 'home', name: 'broken.md', reason: 'yaml: bad' }],
      onclose: () => {},
    });

    // Fix here loads the raw file into the editor (not the OS editor).
    await fireEvent.click(screen.getByRole('button', { name: 'Fix here' }));
    const box = (await screen.findByLabelText(/raw text of broken.md/)) as HTMLTextAreaElement;
    expect(box.value).toContain('<<<<<<<');
    expect(readSkipped).toHaveBeenCalledWith('home', 'broken.md');
    // While markers remain, the live indicator warns.
    expect(screen.getByText(/still has conflict markers/)).toBeTruthy();

    // Editing the markers out flips the indicator — immediate feedback, no save-and-pray.
    await fireEvent.input(box, { target: { value: RESOLVED } });
    expect(screen.getByText(/no conflict markers/)).toBeTruthy();

    // Save writes exactly what the user typed back (byte-for-byte, no side-picking).
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(resolveSkipped).toHaveBeenCalledWith('home', 'broken.md', RESOLVED);
  });

  it('keeps the editor open and warns when the saved text still will not parse', async () => {
    readSkipped.mockResolvedValue({ text: CONFLICTED });
    resolveSkipped.mockResolvedValue({ parses: false });

    render(SkippedPanel, {
      skipped: [{ vault: 'home', name: 'broken.md', reason: 'yaml: bad' }],
      onclose: () => {},
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Fix here' }));
    await screen.findByLabelText(/raw text of broken.md/);
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    // Still unresolved: the editor stays open and says so, rather than silently "succeeding".
    expect(await screen.findByText(/still has conflict markers — remove them and save again/)).toBeTruthy();
    expect(screen.getByLabelText(/raw text of broken.md/)).toBeTruthy();
  });
});

// The status chip keeps rotate as the primary tap and adds a picker as a secondary gesture
// (contextmenu = right-click on a laptop, long-press on a phone), so a distant status is one jump
// away without stepping through the rotation.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import StatusChip from './StatusChip.svelte';

const STATUSES = ['todo', 'doing', 'done'];

describe('StatusChip', () => {
  it('rotates to the next status on a plain click (the fast primary gesture)', async () => {
    const onchange = vi.fn();
    render(StatusChip, { status: 'todo', statuses: STATUSES, onchange });
    await fireEvent.click(screen.getByRole('button', { name: 'status: todo' }));
    expect(onchange).toHaveBeenCalledWith('doing');
  });

  it('opens a picker on contextmenu and jumps straight to the chosen status', async () => {
    const onchange = vi.fn();
    render(StatusChip, { status: 'todo', statuses: STATUSES, onchange });
    await fireEvent.contextMenu(screen.getByRole('button', { name: 'status: todo' }));
    // Jump to 'done' without rotating through 'doing'.
    await fireEvent.click(await screen.findByRole('button', { name: 'done' }));
    expect(onchange).toHaveBeenCalledWith('done');
  });

  it('the picker can clear the status via "none"', async () => {
    const onchange = vi.fn();
    render(StatusChip, { status: 'doing', statuses: STATUSES, onchange });
    await fireEvent.contextMenu(screen.getByRole('button', { name: 'status: doing' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'none' }));
    expect(onchange).toHaveBeenCalledWith(null);
  });
});

// **The roller in the phone's bar** (`roller.ts`): rolled with the thumb, a wheel notch, a tap.
import { render, fireEvent } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import TabBar from './TabBar.svelte';
import { newPane } from './panes';
import { STEP } from './roller';

function bar(focused = 1) {
  const panes = ['Today', 'Field notes', 'A page', 'Music'].map((t) =>
    newPane('note', { noteTitle: t }),
  );
  const onselect = vi.fn();
  const onpair = vi.fn();
  const r = render(TabBar, {
    panes,
    focused,
    besideIndex: -1,
    position: 'bottom',
    roller: true,
    title: (p) => p.noteTitle ?? '',
    icon: () => null,
    onselect,
    onclose: vi.fn(),
    onpair,
    onnew: vi.fn(),
    onopennote: vi.fn(),
    onsettings: vi.fn(),
    onhelp: vi.fn(),
    onbackup: vi.fn(),
    onundo: vi.fn(),
  });
  const ul = r.container.querySelector('ul.roller') as HTMLElement;
  return { ul, onselect, onpair, r, panes };
}

it('shows the open items on the roller, the one in front marked', () => {
  const { ul } = bar(1);
  expect(ul).toBeTruthy();
  const on = ul.querySelector('[aria-current="page"]');
  expect(on?.textContent).toContain('Field notes');
});

it('a roll to the left lands on the next item, and opens it', async () => {
  const { ul, onselect } = bar(1);
  await fireEvent.pointerDown(ul, { clientX: 200, clientY: 10, pointerId: 1 });
  await fireEvent.pointerMove(ul, { clientX: 200 - STEP * 0.6, clientY: 12, pointerId: 1 });
  await fireEvent.pointerMove(ul, { clientX: 200 - STEP * 1.1, clientY: 12, pointerId: 1 });
  await fireEvent.pointerUp(ul, { clientX: 200 - STEP * 1.1, clientY: 12, pointerId: 1 });
  expect(onselect).toHaveBeenCalledWith(2);
});

it('a wheel notch moves one item, either way', async () => {
  const { ul, onselect } = bar(2);
  await fireEvent.wheel(ul, { deltaY: 120 });
  expect(onselect).toHaveBeenLastCalledWith(3);
  await fireEvent.wheel(ul, { deltaY: -120 });
  expect(onselect).toHaveBeenLastCalledWith(1);
});

it('a tap on an item opens it', async () => {
  const { ul, onselect } = bar(1);
  const music = [...ul.querySelectorAll('button.pick')].find((b) =>
    b.textContent?.includes('Music'),
  );
  await fireEvent.click(music as HTMLElement);
  expect(onselect).toHaveBeenCalledWith(3);
});

it('rolls even when the browser refuses to follow the finger', async () => {
  const { ul, onselect } = bar(1);
  ul.setPointerCapture = () => {
    throw new DOMException('Invalid pointer id', 'NotFoundError');
  };
  await fireEvent.pointerDown(ul, { clientX: 200, clientY: 10, pointerId: 9 });
  await fireEvent.pointerMove(ul, { clientX: 200 + STEP * 0.6, clientY: 12, pointerId: 9 });
  await fireEvent.pointerMove(ul, { clientX: 200 + STEP * 1.1, clientY: 12, pointerId: 9 });
  await fireEvent.pointerUp(ul, { clientX: 200 + STEP * 1.1, clientY: 12, pointerId: 9 });
  expect(onselect).toHaveBeenCalledWith(0);
});

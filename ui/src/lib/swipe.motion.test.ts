// The view follows the finger (2026-11-04). Motion is off in every other test, so here it is on.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./charts/motion', () => ({ still: () => false, dur: (ms = 520) => ms }));
import { swipe } from './swipe';

function fire(el: Element, type: string, x: number, y = 300) {
  const e = new Event(type, { bubbles: true, cancelable: true });
  const point = [{ clientX: x, clientY: y }];
  Object.defineProperty(e, type === 'touchend' ? 'changedTouches' : 'touches', { value: point });
  if (type === 'touchend') Object.defineProperty(e, 'touches', { value: [] });
  el.dispatchEvent(e);
}
function screenOf(width = 400) {
  const el = document.createElement('div');
  Object.defineProperty(el, 'clientWidth', { value: width });
  document.body.append(el);
  return el;
}
const x = (el: HTMLElement) => el.style.getPropertyValue('--swipe-x');
let n = 0;
const fresh = () => `pane-${++n}`;

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
});

describe('a swipe, moving', () => {
  it('follows the finger with the next view named beside it, then steps', () => {
    const el = screenOf();
    const onnext = vi.fn();
    swipe(el, { onnext, next: 'Menu', prev: null, key: fresh() });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 180);
    expect(el.classList.contains('swipe-moving')).toBe(true);
    expect(x(el)).toBe('-120px');
    expect(el.querySelector('.swipe-peek')?.textContent).toBe('Menu');
    expect(el.querySelector('.swipe-peek')?.getAttribute('aria-hidden')).toBe('true');
    fire(el, 'touchend', 180);
    expect(x(el)).toBe('-400px');
    expect(onnext).not.toHaveBeenCalled();
    vi.advanceTimersByTime(220);
    expect(onnext).toHaveBeenCalledOnce();
  });

  it('gives a little past either end and comes back, naming nothing and stepping nowhere', () => {
    const el = screenOf();
    const onprev = vi.fn();
    swipe(el, { onprev, prev: null, next: 'Menu', key: fresh() });
    fire(el, 'touchstart', 100);
    fire(el, 'touchmove', 300);
    expect(x(el)).toBe('50px');
    expect(el.querySelector('.swipe-peek')).toBeNull();
    fire(el, 'touchend', 300);
    vi.advanceTimersByTime(300);
    expect(onprev).not.toHaveBeenCalled();
    expect(el.classList.contains('swipe-moving')).toBe(false);
    expect(x(el)).toBe('');
  });

  it('goes back when the drag was too short, and leaves nothing on the element', () => {
    const el = screenOf();
    const onnext = vi.fn();
    swipe(el, { onnext, next: 'Menu', key: fresh() });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 270);
    fire(el, 'touchend', 270);
    vi.advanceTimersByTime(300);
    expect(onnext).not.toHaveBeenCalled();
    expect(el.classList.contains('swipe-moving')).toBe(false);
    expect(el.querySelector('.swipe-peek')).toBeNull();
  });

  it('leaves a vertical move alone, even if it wanders sideways afterwards', () => {
    const el = screenOf();
    swipe(el, { next: 'Menu', key: fresh() });
    fire(el, 'touchstart', 300, 300);
    fire(el, 'touchmove', 298, 340);
    fire(el, 'touchmove', 200, 345);
    expect(el.classList.contains('swipe-moving')).toBe(false);
  });

  it('a view reached from its tab slides in from its side, and one swiped to only appears', () => {
    const key = fresh();
    const order = ['Day', 'Menu', 'Trends'];
    const el = screenOf();
    const action = swipe(el, { key, order, shown: 'Day' });
    expect(el.classList.contains('swipe-moving'), 'first shown: nothing to arrive from').toBe(
      false,
    );
    action.update({ key, order, shown: 'Trends' });
    expect(el.classList.contains('swipe-arriving')).toBe(true);
    expect(el.style.getPropertyValue('--swipe-from')).toBe('56px');
    vi.advanceTimersByTime(200);
    expect(el.classList.contains('swipe-arriving')).toBe(false);
    expect(el.style.getPropertyValue('--swipe-from')).toBe('');

    // Swiped: its name came in, so it does not slide again. The element may be a new one.
    const onnext = vi.fn();
    action.update({ key, order, shown: 'Trends', next: 'x', onnext });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 100);
    fire(el, 'touchend', 100);
    vi.advanceTimersByTime(220);
    expect(onnext).toHaveBeenCalledOnce();
    action.destroy();
    const built = screenOf();
    swipe(built, { key, order, shown: 'Menu' });
    expect(built.classList.contains('swipe-arriving')).toBe(true);
    expect(built.style.getPropertyValue('--swipe-from')).toBe('72px');
  });

  // The owner, 2026-11-05: "an awkward pause with the name of the next subview in the center".
  it('leaves at an even speed, and the next view takes over the same way with the name going', () => {
    const key = fresh();
    const order = ['Day', 'Menu'];
    const el = screenOf();
    const onnext = vi.fn();
    const action = swipe(el, { key, order, shown: 'Day', next: 'Menu', prev: null, onnext });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 100);
    fire(el, 'touchend', 100);
    expect(el.style.getPropertyValue('--swipe-ease'), 'no slowing into a stop').toBe('linear');
    vi.advanceTimersByTime(220);
    expect(onnext).toHaveBeenCalledOnce();

    action.update({ key, order, shown: 'Menu', next: null, prev: 'Day', onnext });
    expect(el.style.getPropertyValue('--swipe-from'), 'from the side it was moving from').toBe(
      '72px',
    );
    expect(el.style.getPropertyValue('--swipe-ease')).not.toBe('linear');
    const going = el.querySelector('.swipe-peek');
    expect(going?.textContent).toBe('Menu');
    expect(going?.classList.contains('swipe-peek-going')).toBe(true);
    vi.advanceTimersByTime(220);
    expect(el.querySelector('.swipe-peek'), 'and leaves nothing behind').toBeNull();
    expect(el.classList.contains('swipe-arriving')).toBe(false);
  });

  it('a swipe to the previous view comes on from the other side', () => {
    const key = fresh();
    const order = ['Day', 'Menu'];
    const el = screenOf();
    const onprev = vi.fn();
    const action = swipe(el, { key, order, shown: 'Menu', prev: 'Day', next: null, onprev });
    fire(el, 'touchstart', 100);
    fire(el, 'touchmove', 300);
    fire(el, 'touchend', 300);
    vi.advanceTimersByTime(220);
    expect(onprev).toHaveBeenCalledOnce();
    action.update({ key, order, shown: 'Day', prev: null, next: 'Menu', onprev });
    expect(el.style.getPropertyValue('--swipe-from')).toBe('-72px');
  });

  it('a view of another group has no side to come from', () => {
    const key = fresh();
    const el = screenOf();
    const action = swipe(el, { key, order: ['Day', 'Menu'], shown: 'Day' });
    action.update({ key, order: ['Week', 'Plan'], shown: 'Plan' });
    expect(el.classList.contains('swipe-arriving')).toBe(false);
  });

  it('a view that never comes does not leave the screen off to the side', () => {
    const el = screenOf();
    swipe(el, { onnext: () => {}, next: 'Menu', key: fresh() });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 100);
    fire(el, 'touchend', 100);
    vi.advanceTimersByTime(220);
    expect(x(el)).toBe('-400px');
    vi.advanceTimersByTime(2500 + 200);
    expect(el.classList.contains('swipe-moving')).toBe(false);
  });

  it('takes nothing away when it is gone', () => {
    const el = screenOf();
    const onnext = vi.fn();
    const action = swipe(el, { onnext, next: 'Menu', key: fresh() });
    fire(el, 'touchstart', 300);
    fire(el, 'touchmove', 100);
    fire(el, 'touchend', 100);
    action.destroy();
    vi.advanceTimersByTime(3000);
    expect(onnext).not.toHaveBeenCalled();
  });
});

// A swipe steps between a group's views on a touch screen, and never takes a sideways move from
// what handles one itself (2026-10-29).
import { afterEach, describe, expect, it, vi } from 'vitest';
import { claimed, stepped, swipe } from './swipe';

function touch(el: Element, type: string, x: number, y: number) {
  const e = new Event(type, { bubbles: true });
  const point = [{ clientX: x, clientY: y }];
  Object.defineProperty(e, type === 'touchend' ? 'changedTouches' : 'touches', { value: point });
  if (type === 'touchend') Object.defineProperty(e, 'touches', { value: [] });
  el.dispatchEvent(e);
}
function drag(el: Element, dx: number, dy = 0) {
  touch(el, 'touchstart', 200, 300);
  touch(el, 'touchend', 200 + dx, 300 + dy);
}
afterEach(() => document.body.replaceChildren());

describe('swipe', () => {
  it('steps to the next view on a swipe left and the previous on a swipe right', () => {
    const screen = document.createElement('div');
    document.body.append(screen);
    const onnext = vi.fn();
    const onprev = vi.fn();
    swipe(screen, { onnext, onprev });
    drag(screen, -120);
    drag(screen, 120);
    expect([onnext.mock.calls.length, onprev.mock.calls.length]).toEqual([1, 1]);
  });

  it('ignores a tap, a short move, a scroll that wandered, and a screen with one view', () => {
    const screen = document.createElement('div');
    document.body.append(screen);
    const onnext = vi.fn();
    const action = swipe(screen, { onnext });
    drag(screen, -20);
    drag(screen, -80, -70);
    action.update({ onnext, enabled: false });
    drag(screen, -150);
    expect(onnext).not.toHaveBeenCalled();
  });

  it('leaves a sideways move to a field, the note text, and what is marked', () => {
    const screen = document.createElement('div');
    for (const html of [
      '<input>',
      '<div class="editor"><p>text</p></div>',
      '<div data-no-swipe><span>x</span></div>',
    ]) {
      screen.innerHTML = html;
      expect(claimed(screen.querySelector('input, p, span'), screen), html).toBe(true);
    }
    screen.innerHTML = '<p>a day</p>';
    expect(claimed(screen.querySelector('p'), screen)).toBe(false);
  });

  it('claims a sideways move, so the browser does not also go back, and never a vertical one', () => {
    const screen = document.createElement('div');
    document.body.append(screen);
    swipe(screen, {});
    const moveTo = (x: number, y: number) => {
      const e = new Event('touchmove', { bubbles: true, cancelable: true });
      Object.defineProperty(e, 'touches', { value: [{ clientX: x, clientY: y }] });
      screen.dispatchEvent(e);
      return e.defaultPrevented;
    };
    touch(screen, 'touchstart', 200, 300);
    expect(moveTo(260, 305)).toBe(true);
    touch(screen, 'touchstart', 200, 300);
    expect(moveTo(210, 400)).toBe(false);
  });

  it('names the view a step lands on, and nothing past either end', () => {
    const views = ['Day', 'Menu', 'Trends', 'Diet'];
    expect(stepped(views, 'Day', 1)).toBe('Menu');
    expect(stepped(views, 'Menu', -1)).toBe('Day');
    expect(stepped(views, 'Diet', 1)).toBeNull();
    expect(stepped(views, 'Day', -1)).toBeNull();
    expect(stepped(views, 'Gone', 1)).toBeNull();
  });
});

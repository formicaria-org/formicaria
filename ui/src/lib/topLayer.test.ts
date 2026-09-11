import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { topLayer } from './topLayer';

// jsdom has no top layer, so these stand one in. They pin where the popup is placed; whether an
// engine then draws it above everything is checked by screenshot.
const W = window.innerWidth;
const H = window.innerHeight;

type Rect = { top: number; bottom: number; left: number; right: number };

function anchorAt(rect: Rect) {
  const a = document.createElement('button');
  a.getBoundingClientRect = () =>
    ({
      ...rect,
      x: rect.left,
      y: rect.top,
      width: rect.right - rect.left,
      height: rect.bottom - rect.top,
      toJSON: () => ({}),
    }) as DOMRect;
  document.body.append(a);
  return a;
}

function popup(width = 208) {
  const p = document.createElement('div');
  Object.defineProperty(p, 'offsetWidth', { configurable: true, get: () => width });
  document.body.append(p);
  return p;
}

const proto = HTMLElement.prototype as unknown as Record<string, unknown>;
const real = { show: proto.showPopover, hide: proto.hidePopover };
let show: ReturnType<typeof vi.fn>;
let hide: ReturnType<typeof vi.fn>;
beforeEach(() => {
  show = vi.fn();
  hide = vi.fn();
  proto.showPopover = show;
  proto.hidePopover = hide;
});
afterEach(() => {
  for (const [key, fn] of [
    ['showPopover', real.show],
    ['hidePopover', real.hide],
  ] as const) {
    if (fn) proto[key] = fn;
    else delete proto[key];
  }
  document.body.replaceChildren();
});

describe('topLayer', () => {
  it('opens below a control in the top half, its right edge under the control’s', () => {
    const p = popup(208);
    topLayer(p, anchorAt({ top: 60, bottom: 88, left: 700, right: 738 }));
    expect(show).toHaveBeenCalledOnce();
    expect(p.getAttribute('popover')).toBe('manual');
    expect(p.dataset.open).toBe('down');
    expect(p.style.top).toBe('92px');
    expect(p.style.getPropertyValue('--opens-at')).toBe('92px');
    expect(p.style.left).toBe(`${738 - 208}px`);
  });

  it('opens above a control in the bottom half', () => {
    const p = popup(208);
    topLayer(p, anchorAt({ top: H - 100, bottom: H - 72, left: 400, right: 438 }));
    expect(p.dataset.open).toBe('up');
    expect(p.style.bottom).toBe('104px');
    expect(p.style.top).toBe('auto');
    expect(p.style.getPropertyValue('--opens-at')).toBe('104px');
  });

  it('never leaves either side of the screen', () => {
    const nearLeft = popup(208);
    topLayer(nearLeft, anchorAt({ top: 10, bottom: 38, left: 20, right: 58 }));
    expect(nearLeft.style.left).toBe('8px');

    const pastRight = popup(208);
    topLayer(pastRight, anchorAt({ top: 10, bottom: 38, left: W + 50, right: W + 90 }));
    expect(pastRight.style.left).toBe(`${W - 8 - 208}px`);
  });

  it('follows its control when the screen changes size, and stops once closed', () => {
    const rect = { top: 60, bottom: 88, left: 600, right: 638 };
    const p = popup(208);
    const action = topLayer(p, anchorAt(rect));

    rect.top = H - 100;
    rect.bottom = H - 72;
    window.dispatchEvent(new Event('resize'));
    expect(p.dataset.open).toBe('up');

    action?.destroy();
    expect(hide).toHaveBeenCalledOnce();
    rect.top = 60;
    rect.bottom = 88;
    window.dispatchEvent(new Event('resize'));
    expect(p.dataset.open).toBe('up');
  });

  it('does nothing where the browser has no top layer', () => {
    delete proto.showPopover;
    const p = popup();
    expect(topLayer(p, anchorAt({ top: 60, bottom: 88, left: 700, right: 738 }))).toBeUndefined();
    expect(p.hasAttribute('popover')).toBe(false);
    expect(p.style.top).toBe('');
  });
});

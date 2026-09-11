import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { popup, type Box } from './popup';

// jsdom has no layout and no top layer, so these stand both in: an anchor reports a box, a popup
// reports a size. They pin where a popup is placed; whether an engine then draws it on top of
// everything is checked by screenshot.
const W = window.innerWidth;
const H = window.innerHeight;

function anchorAt(box: Box) {
  const a = document.createElement('button');
  a.getBoundingClientRect = () =>
    ({
      ...box,
      x: box.left,
      y: box.top,
      width: box.right - box.left,
      height: box.bottom - box.top,
      toJSON: () => ({}),
    }) as DOMRect;
  document.body.append(a);
  return a;
}

function sized(width: number, height: number) {
  const p = document.createElement('ul');
  Object.defineProperty(p, 'offsetWidth', { configurable: true, get: () => width });
  Object.defineProperty(p, 'offsetHeight', { configurable: true, get: () => height });
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
  // A frame, immediately: these tests are about where, not when.
  vi.stubGlobal('requestAnimationFrame', (f: FrameRequestCallback) => (f(0), 1));
  vi.stubGlobal('cancelAnimationFrame', () => {});
});
afterEach(() => {
  for (const [key, fn] of [
    ['showPopover', real.show],
    ['hidePopover', real.hide],
  ] as const) {
    if (fn) proto[key] = fn;
    else delete proto[key];
  }
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

describe('popup', () => {
  it('lifts into the top layer and opens below its control, start edges lined up', () => {
    const p = sized(208, 200);
    popup(p, { anchor: anchorAt({ top: 60, bottom: 88, left: 300, right: 338 }) });
    expect(show).toHaveBeenCalledOnce();
    expect(p.getAttribute('popover')).toBe('manual');
    expect(p.style.position).toBe('fixed');
    expect(p.dataset.open).toBe('down');
    expect(p.style.top).toBe('92px');
    expect(p.style.left).toBe('300px');
  });

  it('lines up end edges when asked', () => {
    const p = sized(208, 200);
    popup(p, { anchor: anchorAt({ top: 60, bottom: 88, left: 300, right: 338 }), align: 'end' });
    expect(p.style.left).toBe(`${338 - 208}px`);
  });

  it('opens above when it does not fit below and there is more room above', () => {
    const p = sized(208, 200);
    popup(p, { anchor: anchorAt({ top: H - 100, bottom: H - 72, left: 300, right: 338 }) });
    expect(p.dataset.open).toBe('up');
    expect(p.style.top).toBe(`${H - 100 - 4 - 200}px`);
  });

  it('opens above when that is preferred and it fits, and below when it does not', () => {
    const fits = sized(208, 100);
    popup(fits, {
      anchor: anchorAt({ top: 300, bottom: 320, left: 100, right: 140 }),
      prefer: 'above',
    });
    expect(fits.dataset.open).toBe('up');

    const tooTall = sized(208, 200);
    popup(tooTall, {
      anchor: anchorAt({ top: 60, bottom: 80, left: 100, right: 140 }),
      prefer: 'above',
    });
    expect(tooTall.dataset.open).toBe('down');
  });

  it('never leaves either side of the screen, and is never wider than it', () => {
    const nearLeft = sized(208, 100);
    popup(nearLeft, {
      anchor: anchorAt({ top: 10, bottom: 38, left: 20, right: 58 }),
      align: 'end',
    });
    expect(nearLeft.style.left).toBe('8px');

    const pastRight = sized(208, 100);
    popup(pastRight, { anchor: anchorAt({ top: 10, bottom: 38, left: W - 30, right: W - 2 }) });
    expect(pastRight.style.left).toBe(`${W - 8 - 208}px`);
    expect(pastRight.style.maxWidth).toBe(`${W - 16}px`);
  });

  it('takes a box, or a function returning one, for a control that is not an element', () => {
    const p = sized(100, 50);
    popup(p, { anchor: { top: 40, bottom: 60, left: 120, right: 120 } });
    expect([p.style.top, p.style.left]).toEqual(['64px', '120px']);

    const q = sized(100, 50);
    popup(q, { anchor: () => ({ top: 10, bottom: 30, left: 50, right: 50 }), gap: 10 });
    expect([q.style.top, q.style.left]).toEqual(['40px', '50px']);
  });

  it('follows its control when the screen changes and when options change, until closed', () => {
    const box = { top: 60, bottom: 88, left: 300, right: 338 };
    const p = sized(208, 200);
    const action = popup(p, { anchor: anchorAt(box) });

    box.top = H - 100;
    box.bottom = H - 72;
    window.dispatchEvent(new Event('resize'));
    expect(p.dataset.open).toBe('up');

    action.update({ anchor: anchorAt({ top: 10, bottom: 30, left: 40, right: 60 }) });
    expect([p.dataset.open, p.style.left]).toEqual(['down', '40px']);

    action.destroy();
    expect(hide).toHaveBeenCalledOnce();
    box.top = 60;
    box.bottom = 88;
    window.dispatchEvent(new Event('resize'));
    expect(p.style.left).toBe('40px');
  });

  it('is still placed where the browser has no top layer', () => {
    delete proto.showPopover;
    const p = sized(208, 200);
    popup(p, { anchor: anchorAt({ top: 60, bottom: 88, left: 300, right: 338 }) });
    expect(p.hasAttribute('popover')).toBe(false);
    expect([p.style.top, p.style.left]).toEqual(['92px', '300px']);
  });

  it('defaults to its parent as the control', () => {
    const parent = anchorAt({ top: 100, bottom: 120, left: 60, right: 90 });
    const p = sized(120, 80);
    parent.append(p);
    popup(p);
    expect([p.style.top, p.style.left]).toEqual(['124px', '60px']);
  });
});

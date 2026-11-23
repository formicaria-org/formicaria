// **The layout guard** (2026-11-23; the owner: "no option should fall out of a screen, using a
// feature should not be hindered by stuff floating"). Run in a page by usability-check.py and
// phone-media-check.py after their steps. Answers a list of problems, empty when the screen is right:
// - a control (button, link, field, menu item) outside the screen that nothing scrolls to;
// - a control cut by a box that does not scroll;
// - a control with something else lying over its middle;
// - with an editor focused, the line being typed hidden or covered (`caret`).
// The viewport is the visual one, so a phone's keyboard counts as covering what is under it.
return (() => {
  const vv = window.visualViewport;
  const W = vv ? vv.width + vv.offsetLeft : innerWidth;
  // The phone's keyboard is drawn over the app (Android, edge to edge): the app is told its height as
  // `--kb` (`lib/keyboard.ts`), and what lies under it is not on the screen.
  const kb = parseFloat(document.documentElement.style.getPropertyValue('--kb')) || 0;
  const H = (vv ? vv.height + vv.offsetTop : innerHeight) - kb;
  const SEL =
    'button, a[href], input:not([type=hidden]), select, textarea, [role=button], [role=menuitem], [role=checkbox], summary';
  const name = (el) =>
    (el.getAttribute('aria-label') || el.textContent || el.getAttribute('title') || el.tagName)
      .trim()
      .replace(/\s+/g, ' ')
      .slice(0, 50);
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return false;
    for (let e = el; e; e = e.parentElement) {
      const s = getComputedStyle(e);
      if (s.display === 'none' || s.visibility === 'hidden' || Number(s.opacity) === 0) return false;
    }
    return true;
  };
  const scrolls = (s, axis) => /(auto|scroll)/.test(axis === 'x' ? s.overflowX : s.overflowY);
  const box = (e) =>
    e.tagName.toLowerCase() + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\s+/)[0] : '');
  const out = [];
  for (const el of document.querySelectorAll(SEL)) {
    if (!visible(el) || el.closest('[aria-hidden="true"], .sr-only, .visually-hidden')) continue;
    const r = el.getBoundingClientRect();
    // Walk out to the first box that scrolls: whatever it does not show is reached by scrolling it,
    // so a box further out cannot cut this control off. Boxes before it that clip, cut.
    let reachX = false;
    let shown = { left: 0, top: 0, right: W, bottom: H };
    for (let e = el.parentElement; e && e !== document.documentElement; e = e.parentElement) {
      const s = getComputedStyle(e);
      const b = e.getBoundingClientRect();
      const clips = /(hidden|clip|auto|scroll)/.test(s.overflowX + s.overflowY);
      if (clips) {
        shown = {
          left: Math.max(shown.left, b.left),
          top: Math.max(shown.top, b.top),
          right: Math.min(shown.right, b.right),
          bottom: Math.min(shown.bottom, b.bottom),
        };
      }
      if (scrolls(s, 'x') || scrolls(s, 'y')) {
        if (scrolls(s, 'x')) reachX = true;
        break;
      }
      const cut = r.right > b.right + 1 || r.left < b.left - 1 || r.bottom > b.bottom + 1 || r.top < b.top - 1;
      if (clips && cut) {
        out.push(`cut off by ${box(e)}: "${name(el)}"`);
        break;
      }
    }
    if (!reachX && (r.left < -1 || r.right > W + 1)) out.push(`outside the screen: "${name(el)}"`);
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;
    // Only what is in view can be covered: a control scrolled out of its box is not.
    if (cx > shown.left && cx < shown.right && cy > shown.top && cy < shown.bottom) {
      const top = document.elementFromPoint(cx, cy);
      if (top && !el.contains(top) && !top.contains(el) && !(el.labels && [...el.labels].some((l) => l.contains(top))))
        out.push(`covered by ${box(top)} "${name(top)}": "${name(el)}"`);
    }
  }
  const ed = document.activeElement && document.activeElement.closest && document.activeElement.closest('.cm-editor, textarea, input');
  if (ed) {
    const s = getSelection();
    const c = s && s.rangeCount ? s.getRangeAt(0).getBoundingClientRect() : null;
    if (!c || (c.width === 0 && c.height === 0)) {
      const b = ed.getBoundingClientRect();
      if (b.bottom < 0 || b.top > H) out.push('caret: the field being typed in is off the screen');
    } else {
      if (c.bottom > H + 1 || c.top < -1) out.push(`caret: the line being typed is off the screen (${Math.round(c.top)}–${Math.round(c.bottom)} of ${Math.round(H)})`);
      const at = document.elementFromPoint(Math.min(Math.max(c.left + 1, 1), W - 1), c.top + c.height / 2);
      if (at && !ed.contains(at)) out.push(`caret: the line being typed is covered by "${name(at)}"`);
    }
  }
  return [...new Set(out)];
})();

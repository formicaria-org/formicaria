// A deterministic hue for a name — so the same identity reads the same colour everywhere. Used
// for vault badges (which audience) and for contributors (who edited), so a person's colour on a
// note's "edited by" label matches their activity rows and their filter chip. Only a hue is
// derived here; the badge components fix saturation/lightness so the label stays legible on every
// hue in both light and dark themes. A hash → hue handles any name, including ones no theme has
// ever heard of (which is why this replaced a theme keyed off a data attribute).

/** A stable hue in [0, 360) for a name. Same name → same hue, always. */
export function hashHue(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) {
    h = (h * 31 + name.charCodeAt(i)) >>> 0;
  }
  return h % 360;
}

// **Every theme is legible** (2026-11-23; the owner: *"go through the current suggested themes and
// see if things are well visible in them"*). Each built-in theme and each starter, in light and
// dark, measured by WCAG 2.2's contrast ratio for the pairs the app draws text with: 4.5:1 for
// text (1.4.3), 3:1 for focus rings and for colours that mark rather than read (1.4.11).
// The UI's tsconfig has no node types, on purpose (`panes.test.ts`); this test reads one file.
// @ts-expect-error -- `node:fs`, untyped here
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { STARTERS } from './starterThemes';

// Read from disk: the test setup turns CSS imports into nothing, `?raw` included.
declare const process: { cwd(): string; stdout: { write(s: string): void } };
const appCss: string = readFileSync(`${process.cwd()}/src/app.css`, 'utf8');

/** `--name: value;` pairs inside the block that follows `selector`. */
function block(css: string, selector: string): Record<string, string> {
  const at = css.indexOf(selector);
  if (at < 0) return {};
  const open = css.indexOf('{', at);
  const close = css.indexOf('}', open);
  const out: Record<string, string> = {};
  for (const m of css.slice(open + 1, close).matchAll(/--([\w-]+):\s*([^;]+);/g))
    out[m[1]] = m[2].trim();
  return out;
}
if (!appCss.includes('--surface')) throw new Error('app.css was not read');
const BASE = {
  dark: block(appCss, ":where(:root[data-theme='dark']) {"),
  light: block(appCss, ":where(:root[data-theme='light']) {"),
};

/** A theme as the page has it: the base for the mode, the theme's own tokens over it. */
function themeOf(name: string, mode: 'light' | 'dark'): Record<string, string> {
  const starter = STARTERS.find((s) => s.name === name);
  const own = starter
    ? { ...block(starter.css, ':root {'), ...block(starter.css, `:root[data-theme='${mode}'] {`) }
    : {};
  const t: Record<string, string> = { ...BASE[mode], ...own };
  // `var(--x)` within the theme, resolved.
  for (const k of Object.keys(t)) {
    let v = t[k];
    for (let i = 0; i < 5 && v.startsWith('var(--'); i++) v = t[v.slice(6, v.indexOf(')'))] ?? v;
    t[k] = v;
  }
  return t;
}

function rgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h.slice(0, 6);
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)) as [number, number, number];
}
function luminance(hex: string): number {
  const [r, g, b] = rgb(hex).map((c) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** The pairs the app draws: [text, behind it, the least ratio]. */
const PAIRS: [string, string, number][] = [
  ['text', 'bg', 4.5],
  ['text', 'surface', 4.5],
  ['text', 'surface-elevated', 4.5],
  ['text', 'surface-hover', 4.5],
  ['text-muted', 'bg', 4.5],
  ['text-muted', 'surface', 4.5],
  ['text-muted', 'surface-elevated', 4.5],
  ['text-subtle', 'surface', 3],
  ['link', 'bg', 4.5],
  ['link', 'surface', 4.5],
  ['accent', 'surface', 4.5],
  ['accent', 'bg', 4.5],
  ['text', 'accent-subtle', 4.5],
  ['accent-contrast', 'accent-hover', 4.5],
  ['text-muted', 'surface-hover', 4.5],
  ['link', 'surface-elevated', 4.5],
  ['accent-contrast', 'accent', 4.5],
  ['danger-fg', 'danger-bg', 4.5],
  ['danger-fg', 'surface', 4.5],
  ['ok-fg', 'ok-bg', 4.5],
  ['u-overdue', 'surface', 4.5],
  ['u-soon', 'surface', 4.5],
  ['u-week', 'surface', 4.5],
  ['u-later', 'surface', 4.5],
  ['u-none', 'surface', 4.5],
  ['tint-a', 'surface', 3],
  ['tint-b', 'surface', 3],
  ['tint-c', 'surface', 3],
  ['focus-ring', 'bg', 3],
  ['focus-ring', 'surface', 3],
];

const THEMES = ['Built-in', ...STARTERS.map((s) => s.name)];

it('every theme, light and dark, draws its text legibly', () => {
  const fails: string[] = [];
  for (const name of THEMES)
    for (const mode of ['light', 'dark'] as const) {
      const t = themeOf(name, mode);
      for (const [fg, bg, least] of PAIRS) {
        if (!t[fg]?.startsWith('#') || !t[bg]?.startsWith('#')) continue;
        const r = contrast(t[fg], t[bg]);
        if (r < least)
          fails.push(
            `${name} (${mode}): --${fg} on --${bg} is ${r.toFixed(2)}:1, under ${least}:1`,
          );
      }
    }
  if (fails.length) process.stdout.write(`THEME-FAILS\n${fails.join('\n')}\n`);
  expect(fails).toEqual([]);
});

/** `hsl(h s% l%)` as `#rrggbb`. */
function hsl(h: number, s: number, l: number): string {
  const a = (s / 100) * Math.min(l / 100, 1 - l / 100);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const c = l / 100 - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(c * 255)
      .toString(16)
      .padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}
/** A colour with alpha `a`, laid over `under`. */
function over(top: string, a: number, under: string): string {
  const [t, u] = [rgb(top), rgb(under)];
  return `#${t
    .map((c, i) =>
      Math.round(c * a + u[i] * (1 - a))
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}
/** `S% L%` or `S% L% / A`, as numbers. */
function sla(v: string): [number, number, number] {
  const m = /([\d.]+)%\s+([\d.]+)%(?:\s*\/\s*([\d.]+))?/.exec(v ?? '');
  return m ? [Number(m[1]), Number(m[2]), m[3] ? Number(m[3]) : 1] : [0, 0, 1];
}

it('status chips and column titles are legible in every theme, at every hue', () => {
  const fails = new Set<string>();
  for (const name of THEMES)
    for (const mode of ['light', 'dark'] as const) {
      const t = themeOf(name, mode);
      const [bs, bl, ba] = sla(t['chip-bg']);
      const [fs, fl] = sla(t['chip-fg']);
      const [cs, cl] = sla(t['column-fg']);
      for (let h = 0; h < 360; h += 5) {
        const chip = over(hsl(h, bs, bl), ba, t.surface);
        const r = contrast(hsl(h, fs, fl), chip);
        if (r < 4.5) fails.add(`${name} (${mode}): a chip's text, ${r.toFixed(2)}:1 at hue ${h}`);
        for (const under of ['surface', 'bg']) {
          const c = contrast(hsl(h, cs, cl), t[under]);
          if (c < 4.5)
            fails.add(
              `${name} (${mode}): a column title on --${under}, ${c.toFixed(2)}:1 at hue ${h}`,
            );
        }
      }
    }
  if (fails.size) process.stdout.write(`CHIP-FAILS\n${[...fails].join('\n')}\n`);
  expect([...fails]).toEqual([]);
});

it('each theme tells the browser whether its ground is light or dark', () => {
  expect(appCss).toMatch(
    /:where\(:root\[data-theme='dark'\]\) \{\s*(?:\/\*[\s\S]*?\*\/\s*)?color-scheme: dark;/,
  );
  expect(appCss).toMatch(/:where\(:root\[data-theme='light'\]\) \{\s*color-scheme: light;/);
  expect(STARTERS.find((s) => s.name === 'Paper')?.css).toContain('color-scheme: light;');
  expect(STARTERS.find((s) => s.name === 'Night ink')?.css).toContain('color-scheme: dark;');
  // Buttons and fields take the theme's text colour, not the browser's.
  expect(appCss).toMatch(/button,\s*input,\s*select,\s*textarea \{\s*color: inherit;/);
});

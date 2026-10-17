/// **Themes to start from** (`decisions.md` 2026-10-18, *a theme wins in light mode too, and four
/// starters are offered*). Picking one copies it into the vault as an ordinary theme, which the
/// person then owns and may edit; nothing here is in the vault until it is chosen.
///
/// Each uses only the names in `appearance.ts`'s `SUPPORTED` — the ones the app promises not to
/// move — so a starter never depends on something the manual does not promise. A test checks it.

export interface Starter {
  /** The theme's name, and so its file in the vault (`themes/<slug>.css`). */
  name: string;
  /** One line, in the person's words, under the name. */
  says: string;
  css: string;
}

export const STARTERS: readonly Starter[] = [
  {
    name: 'Paper',
    says: 'Warm off-white and a serif, like a printed page. Light, whichever mode is chosen.',
    css: `/* Paper — warm off-white and a serif. Light, whichever mode is chosen. */
:root {
  --bg: #f4efe6;
  --surface: #fbf8f2;
  --surface-elevated: #fffdf9;
  --surface-hover: #ece5d8;
  --border: #ddd3c2;
  --border-strong: #c4b7a1;
  --text: #2b2621;
  --text-muted: #5e554b;
  --text-subtle: #8a7f72;
  --accent: #8a4b2a;
  --accent-hover: #733c20;
  --accent-contrast: #ffffff;
  --accent-subtle: #f1e2d6;
  --link: #8a4b2a;
  --focus-ring: #8a4b2a;
  --shadow-sm: 0 1px 2px rgb(60 40 20 / 0.08);
  --shadow-md: 0 4px 12px rgb(60 40 20 / 0.12);
  --shadow-lg: 0 16px 48px rgb(60 40 20 / 0.2);
  --tint-a: #2f5f9e;
  --tint-b: #3d7a4a;
  --tint-c: #a06a14;
  --u-overdue: #b8322a;
  --u-soon: #a06a14;
  --u-week: #2f5f9e;
  --u-later: #3d7a4a;
  --u-none: #5e554b;
  --danger-bg: #f6dfd8;
  --danger-fg: #7d2416;
  --ok-bg: #e1eedc;
  --ok-fg: #2c5a2e;
  --font-sans: ui-serif, Georgia, 'Iowan Old Style', 'Times New Roman', serif;
}
`,
  },
  {
    name: 'Night ink',
    says: 'Deep blue-black with a soft blue accent, easy on the eyes at night. Dark, always.',
    css: `/* Night ink — deep blue-black, a soft blue accent. Dark, whichever mode is chosen. */
:root {
  --bg: #0e1320;
  --surface: #141a2a;
  --surface-elevated: #1a2134;
  --surface-hover: #222a40;
  --border: #252e45;
  --border-strong: #35405c;
  --text: #dfe5f2;
  --text-muted: #9aa5bf;
  --text-subtle: #6b7591;
  --accent: #5b8def;
  --accent-hover: #7fa6f5;
  --accent-contrast: #0e1320;
  --accent-subtle: #1b2846;
  --link: #8fb0f7;
  --focus-ring: #7fa6f5;
  --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.35);
  --shadow-md: 0 4px 12px rgb(0 0 0 / 0.4);
  --shadow-lg: 0 16px 48px rgb(0 0 0 / 0.55);
  --tint-a: #7fb2ff;
  --tint-b: #7fcf9f;
  --tint-c: #e0b25a;
  --u-overdue: #ff7d7d;
  --u-soon: #e0b25a;
  --u-week: #7fb2ff;
  --u-later: #7fcf9f;
  --u-none: #9aa5bf;
  --danger-bg: #4a1f2b;
  --danger-fg: #ffd9df;
  --ok-bg: #1d3a2b;
  --ok-fg: #c8f0d8;
}
`,
  },
  {
    name: 'High contrast',
    says: 'Strong text on plain backgrounds, with clear borders. Follows dark and light.',
    css: `/* High contrast — strong text, plain backgrounds, clear borders. Follows dark and light. */
:root[data-theme='dark'] {
  --bg: #000000;
  --surface: #0a0a0a;
  --surface-elevated: #141414;
  --surface-hover: #1f1f1f;
  --border: #6b6b6b;
  --border-strong: #a3a3a3;
  --text: #ffffff;
  --text-muted: #e0e0e0;
  --text-subtle: #bdbdbd;
  --accent: #ffd400;
  --accent-hover: #ffe866;
  --accent-contrast: #000000;
  --accent-subtle: #332b00;
  --link: #ffe866;
  --focus-ring: #ffd400;
  --tint-a: #7cc4ff;
  --tint-b: #7dffa8;
  --tint-c: #ffd400;
  --u-overdue: #ff7a7a;
  --u-soon: #ffd400;
  --u-week: #7cc4ff;
  --u-later: #7dffa8;
  --u-none: #e0e0e0;
  --danger-bg: #5c0010;
  --danger-fg: #ffffff;
  --ok-bg: #003d1a;
  --ok-fg: #ffffff;
}
:root[data-theme='light'] {
  --bg: #ffffff;
  --surface: #ffffff;
  --surface-elevated: #ffffff;
  --surface-hover: #ebebeb;
  --border: #595959;
  --border-strong: #1a1a1a;
  --text: #000000;
  --text-muted: #1f1f1f;
  --text-subtle: #404040;
  --accent: #0033cc;
  --accent-hover: #002699;
  --accent-contrast: #ffffff;
  --accent-subtle: #dce4ff;
  --link: #0033cc;
  --focus-ring: #0033cc;
  --tint-a: #0047b3;
  --tint-b: #006b2e;
  --tint-c: #8a5a00;
  --u-overdue: #b30000;
  --u-soon: #8a5a00;
  --u-week: #0047b3;
  --u-later: #006b2e;
  --u-none: #1f1f1f;
  --danger-bg: #ffe0e0;
  --danger-fg: #7a0000;
  --ok-bg: #dcf5e4;
  --ok-fg: #00451d;
}
`,
  },
  {
    name: 'Large and calm',
    says: 'Bigger text, more room between lines, a shorter line to read. Keeps your colours.',
    css: `/* Large and calm — bigger text, more room between lines, a shorter line. Keeps your colours. */
:root {
  --text-xs: 0.875rem;
  --text-sm: 0.9375rem;
  --text-base: 1.0625rem;
  --text-md: 1.125rem;
  --text-lg: 1.25rem;
  --text-xl: 1.5rem;
  --text-2xl: 1.875rem;
  --lh-base: 1.65;
  --lh-md: 1.8;
  --measure: 60ch;
}
`,
  },
];

/// The file name a theme's name becomes — the rule `vaultfile::slug` applies on the server, so the
/// list (which reports file names) can be matched against a name before it is saved.
export function slug(name: string): string {
  return name
    .trim()
    .replace(/[^\p{L}\p{N} _-]/gu, '-')
    .split(/\s+/)
    .filter(Boolean)
    .join('-')
    .toLowerCase();
}

/// The name a starter is saved under: its own, or with a number when the vault already holds a
/// theme of that name — so choosing a starter never overwrites a theme someone has edited.
/// `taken` is the existing themes' file names (slugs, as `listThemes` reports them).
export function freeName(name: string, taken: readonly string[]): string {
  const used = new Set(taken);
  if (!used.has(slug(name))) return name;
  for (let i = 2; ; i++) {
    const next = `${name} ${i}`;
    if (!used.has(slug(next))) return next;
  }
}

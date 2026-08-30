/// **Applying a user's theme, and being able to get back out of one.**
///
/// `ui/src/app.css` was built for this from the start — *"three layers, so a theme is one file and
/// renderers stay literal-free... re-skinning needs zero renderer edits"* — and `MASTERPLAN.md` has
/// listed `themes/*.css` in the vault layout since the beginning. The file half lives in
/// `crates/fm-app/src/themes.rs`; this is the half that puts the text on the page.
///
/// Pure except for `apply`/`clear` and the arm/disarm pair, so the parts that decide anything can
/// be tested without a DOM.

/// **The supported surface: 59 custom properties are an API; the DOM is not.**
///
/// These are `app.css` layer (2) — the semantic tokens each built-in theme defines — plus the
/// layer (1) scales that are meaningful to re-value. Everything else is deliberately outside the
/// promise: the **legacy aliases** in layer (3) (`--card-bg`, `--panel`, `--tag-bg`, …) are
/// indirections onto these and will be retired; the layout internals (`--rail-w`, `--header-h`) and
/// the device insets (`--safe-*`) are not decoration; `--bp-narrow` is documentation, because CSS
/// cannot interpolate a custom property into a media query. A theme file may of course set any of
/// them — it is CSS — but only this list is what the app promises not to move under it.
export const SUPPORTED: readonly string[] = [
  // Surfaces and text
  'bg', 'surface', 'surface-elevated', 'surface-hover',
  'border', 'border-strong',
  'text', 'text-muted', 'text-subtle',
  // Accent and interaction
  'accent', 'accent-hover', 'accent-contrast', 'accent-subtle', 'link', 'focus-ring',
  'shadow-sm', 'shadow-md', 'shadow-lg',
  // Generic tints, and the derived-urgency palette (never a stored priority)
  'tint-a', 'tint-b', 'tint-c',
  'u-overdue', 'u-soon', 'u-week', 'u-later', 'u-none',
  'danger-bg', 'danger-fg', 'ok-bg', 'ok-fg',
  // Type
  'font-sans', 'font-mono', 'measure',
  'text-xs', 'text-sm', 'text-base', 'text-md', 'text-lg', 'text-xl', 'text-2xl',
  'lh-xs', 'lh-sm', 'lh-base', 'lh-md', 'lh-lg', 'lh-xl', 'lh-2xl',
  // Shape and rhythm
  'radius-sm', 'radius-md', 'radius-lg', 'radius-pill',
  'space-1', 'space-2', 'space-3', 'space-4', 'space-5', 'space-6', 'space-7', 'space-8',
];

const SUPPORTED_SET = new Set(SUPPORTED);

/// What the Settings form can say. Anything a person wants beyond this is the theme file itself —
/// the form is three questions, not a language.
export interface AppearancePrefs {
  accent?: string;
  textBase?: string;
  fontSans?: string;
}

/// Which theme is switched on. **Per-browser, never in the vault**: the theme file travels with the
/// notes so it arrives on the other machine, but *wearing* it is this device's business — the same
/// split the dark/light toggle already has, and the reason a phone and a big monitor can differ.
export interface Selection {
  vault: string;
  name: string;
}

const SELECTED = 'fm-appearance';
const ARMED = 'fm-appearance-armed';
const STYLE_ID = 'fm-theme';

/// The canonical block the form writes. One `:root` rule, one declaration per line, nothing else —
/// which is what makes `fromCss` able to tell "the form wrote this" from "a person did".
export function toCss(prefs: AppearancePrefs): string {
  const decls: string[] = [];
  if (prefs.accent) {
    // One colour answers four tokens. Deriving hover and link from it would need `color-mix()`,
    // whose support across the Android WebViews this ships to is unverified — so they are written
    // explicitly and identically, which is legible and cannot silently degrade.
    for (const t of ['accent', 'accent-hover', 'link', 'focus-ring']) {
      decls.push(`  --${t}: ${prefs.accent};`);
    }
  }
  if (prefs.textBase) decls.push(`  --text-base: ${prefs.textBase};`);
  if (prefs.fontSans) decls.push(`  --font-sans: ${prefs.fontSans};`);
  return decls.length ? `:root {\n${decls.join('\n')}\n}\n` : '';
}

/// Read a theme file back into the form, and say whether the form can faithfully rewrite it.
///
/// **`extra` is the whole point.** A file carrying a selector, a media query, or a token the form
/// does not own cannot be round-tripped through three inputs, so the form must show it and refuse
/// to overwrite it — exactly what `views::save_view` does with a filter richer than one tag, and
/// for exactly the same reason: silently flattening someone's work is worse than declining to.
export function fromCss(text: string): { prefs: AppearancePrefs; extra: boolean } {
  const prefs: AppearancePrefs = {};
  const stripped = text.replace(/\/\*[\s\S]*?\*\//g, '').trim();
  if (!stripped) return { prefs, extra: false };

  const m = stripped.match(/^:root\s*\{([\s\S]*?)\}$/);
  if (!m) return { prefs, extra: true };

  let extra = false;
  for (const raw of m[1].split(';')) {
    const line = raw.trim();
    if (!line) continue;
    const decl = line.match(/^--([a-z0-9-]+)\s*:\s*(.+)$/i);
    if (!decl || !SUPPORTED_SET.has(decl[1])) {
      extra = true;
      continue;
    }
    const [, token, value] = decl;
    if (token === 'accent') prefs.accent = value.trim();
    else if (token === 'text-base') prefs.textBase = value.trim();
    else if (token === 'font-sans') prefs.fontSans = value.trim();
    else if (!['accent-hover', 'link', 'focus-ring'].includes(token)) extra = true;
  }
  return { prefs, extra };
}

/// Put the theme on the page. `textContent`, not `innerHTML` — this is not an HTML sink, so it is
/// outside what `ci/checks.sh` guards and outside what could ever execute.
export function apply(css: string): void {
  let el = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement('style');
    el.id = STYLE_ID;
    document.head.appendChild(el);
  }
  el.textContent = css;
}

export function clear(): void {
  document.getElementById(STYLE_ID)?.remove();
}

// ---------------------------------------------------------------------------------------------
// The escape hatch.
//
// A theme is arbitrary CSS in someone's own vault, and there is **no CSS-level guarantee** against
// one that hides every control — `!important` at equal specificity beats anything we write. So the
// layer that actually rescues the app does not depend on CSS at all.
//
// Before applying a stored theme we set a flag, and we clear it on the **first thing the user
// does**. "The user could see something and touch it" is a far better proxy for "the theme did not
// break the app" than "the script ran", because CSS cannot stop the script from running. If the app
// starts and the flag is still set from last time, the theme is not applied and we say so.
//
// The recovery gesture is therefore *close it and open it again* — which works on Android where
// there is no address bar, and on a phone where there is no keyboard.
//
// **Its honest limit:** a theme that breaks only a screen you reach later is not caught by this.
// Previewing before keeping is what covers that, and it covers most of it.
// ---------------------------------------------------------------------------------------------

const INTERACTIONS = ['pointerdown', 'keydown', 'wheel', 'touchstart', 'scroll'] as const;

function safeGet(key: string): string | null {
  // Storage throws in a private window and in some embedded WebViews. A theme is a nicety; failing
  // to read one must never be what stops the app from starting.
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function safeSet(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* nothing to do: the theme simply will not persist */
  }
}
function safeRemove(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    /* as above */
  }
}

/** Was the previous run still un-interacted-with when it ended? */
export function wasArmed(): boolean {
  return safeGet(ARMED) === '1';
}

/// Arm before applying a stored theme, and disarm on the first sign of life.
///
/// `onDisarm` fires at the same moment, which is what lets the UI show its escape control **only
/// while the theme is still unproven** and drop it the instant the user demonstrably reached
/// something. A permanent floating button would tax every session to insure against a rare one.
export function arm(onDisarm?: () => void): void {
  safeSet(ARMED, '1');
  const off = () => {
    safeRemove(ARMED);
    for (const ev of INTERACTIONS) document.removeEventListener(ev, off, true);
    onDisarm?.();
  };
  // Capture phase, on the document: a theme cannot put anything in the way of this.
  for (const ev of INTERACTIONS) document.addEventListener(ev, off, true);
}

export function disarm(): void {
  safeRemove(ARMED);
}

export function readSelection(): Selection | null {
  const raw = safeGet(SELECTED);
  if (!raw) return null;
  try {
    const v = JSON.parse(raw);
    return typeof v?.name === 'string' ? { vault: String(v.vault ?? ''), name: v.name } : null;
  } catch {
    return null;
  }
}

export function writeSelection(sel: Selection | null): void {
  if (sel) safeSet(SELECTED, JSON.stringify(sel));
  else safeRemove(SELECTED);
}

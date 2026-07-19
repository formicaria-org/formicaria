// Keyboard commands: fixed set, default bindings, user-changeable.
//
// **Bounded on purpose.** The commands are a closed list in this file — a binding can be
// rebound, but a *new* command cannot be invented from Settings. That is the line that keeps
// this from becoming the config system `plan.md` rejects: the palette remains the open-ended
// surface (it is searchable and gains entries for free), and this is only about reaching a
// handful of them without leaving the keyboard.
//
// **Stored where the theme is**, in this browser, touching no vault. That is what lets these
// live in Settings without making Settings a form for vault configuration — the same
// justification the Layout control already stands on.

/** Every command that can carry a binding. Closed set; see the module note. */
export type Command =
  | 'palette'
  | 'nextPane'
  | 'prevPane'
  | 'newNote'
  | 'newView'
  | 'focusSearch'
  | 'closePane'
  | 'backup';

export interface Binding {
  /** `KeyboardEvent.key`, lower-cased for letters. Empty string = unbound. Kept for display,
   *  and used for matching when no `code` was recorded. */
  key: string;
  /** `KeyboardEvent.code` — the key's *physical position*, which does not change with the
   *  keyboard layout. Preferred for matching; absent on the built-in defaults, which are chosen
   *  to be typeable everywhere, and on bindings saved before this field existed. */
  code?: string;
  /** Ctrl on Linux/Windows, Cmd on macOS — they are the same intent, so one flag. */
  mod?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export const LABELS: Record<Command, string> = {
  palette: 'Command palette',
  nextPane: 'Next view',
  prevPane: 'Previous view',
  newNote: 'New note',
  newView: 'New view',
  focusSearch: 'Search',
  closePane: 'Close this view',
  backup: 'Back up',
};

/// **Commands that still fire while you are typing.**
///
/// The reason this set exists at all: `onGlobalKey` ignores keys aimed at an input or textarea,
/// which is right for bare letters — `c` must type a `c`, not create a note. But it also meant
/// that with a note open for editing there was **no way to reach another view without closing
/// it**, which is exactly the complaint that prompted this. Navigation and the palette are not
/// text, so they are exempt; anything that would swallow a keystroke you meant to type is not.
///
/// Every member here must require a modifier, or it would eat typing.
export const WHILE_TYPING: ReadonlySet<Command> = new Set<Command>([
  'palette',
  'nextPane',
  'prevPane',
  'closePane',
]);

/// Defaults chosen to be typeable on a **non-US keyboard**.
///
/// The first version used `Ctrl+[` and `Ctrl+]`, which is what editors have used for decades —
/// and on an Italian layout those brackets need AltGr, so the shortcut was unreachable. Anything
/// that lives behind AltGr, or moves between layouts, is off the table:
///
/// - **`.` and `,`** sit on the same unshifted keys across US, Italian, German, French and
///   Spanish layouts, and are adjacent, so "next/previous" reads physically.
/// - **Digits** are unshifted on Italian (unlike letters-with-accents) and stable everywhere.
/// - Avoided: `[` `]` `/` `\` `;` `'` (AltGr or relocated), Ctrl+Tab and Ctrl+PageUp/Down (the
///   browser keeps those for its own tabs), Alt+Arrow (browser back/forward), and bare
///   Ctrl+Arrow (moves by word inside the editor, which these must not disturb).
/// **The browser gets first refusal, and it does not give these back.** `Ctrl` plus a digit
/// switches tabs; `Ctrl+0` resets zoom; `Ctrl` with `+`/`-` zooms. The first version bound
/// `Ctrl+1/2/9/0` — chosen because digits are stable across layouts — and every one of them was
/// already spoken for, so pressing them zoomed or changed tab instead. Layout-safe is necessary
/// and not sufficient: it also has to be a combination nothing above us has claimed.
const BROWSER_RESERVED = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '-', '=', 'tab'];

/// Deliberately few. Everything else lives in the command palette, which is searchable, grouped,
/// and gains entries for free — binding a key to each would be a second menu to keep in step
/// with the first. These are the ones worth reaching without leaving the keyboard; the rest can
/// be bound in Settings by anyone who wants them.
export const DEFAULTS: Record<Command, Binding> = {
  palette: { key: 'k', mod: true },
  // `.` and `,` sit on the same unshifted keys across US, Italian, German, French and Spanish,
  // are adjacent so the direction reads physically, and no browser claims them.
  nextPane: { key: '.', mod: true },
  prevPane: { key: ',', mod: true },
  newNote: { key: 'c' },
  focusSearch: { key: '/' },
  // Unbound by default: reachable from the palette, and there is no safe, layout-portable
  // combination left that is worth spending on a command used once a session.
  newView: { key: '' },
  closePane: { key: '' },
  backup: { key: '' },
};

/** Is this binding one the browser will take before the page sees it? */
export function reserved(b: Binding): boolean {
  return !!b.mod && !b.alt && !b.shift && BROWSER_RESERVED.includes(b.key.toLowerCase());
}

const STORAGE = 'fm-keys';

/** The bindings in force: defaults, with any the user changed layered on top. */
export function load(): Record<Command, Binding> {
  const out = { ...DEFAULTS };
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE) ?? 'null');
    if (raw && typeof raw === 'object') {
      for (const c of Object.keys(DEFAULTS) as Command[]) {
        const b = raw[c];
        // Validated rather than trusted: this is hand-editable storage, and a malformed entry
        // must fall back to the default rather than silently unbind a command.
        if (b && typeof b.key === 'string') {
          out[c] = {
            key: b.key,
            code: typeof b.code === 'string' ? b.code : undefined,
            mod: !!b.mod,
            alt: !!b.alt,
            shift: !!b.shift,
          };
        }
      }
    }
  } catch {
    /* unreadable storage — the defaults are a complete, working set */
  }
  return out;
}

export function save(bindings: Record<Command, Binding>): void {
  try {
    localStorage.setItem(STORAGE, JSON.stringify(bindings));
  } catch {
    /* private mode — the change just will not persist */
  }
}

export function reset(): void {
  try {
    localStorage.removeItem(STORAGE);
  } catch {
    /* nothing to remove */
  }
}

/** Does this event match this binding?
 *
 *  Matches on `code` — the **physical key** — when the binding recorded one, and falls back to
 *  `key` otherwise. That is what makes a rebind survive a layout: `KeyboardEvent.key` is the
 *  character produced, so a binding captured on one layout can be untypeable on another, while
 *  `code` names the key's position and does not move. Bindings from before this existed carry
 *  no `code` and keep matching by character, which is what they always did. */
export function matches(e: KeyboardEvent, b: Binding): boolean {
  if (!b.key && !b.code) return false; // explicitly unbound
  const mod = e.metaKey || e.ctrlKey;
  const same = b.code ? e.code === b.code : e.key.toLowerCase() === b.key.toLowerCase();
  return same && mod === !!b.mod && e.altKey === !!b.alt && e.shiftKey === !!b.shift;
}

/** A binding as a human reads it: `Ctrl+Shift+W`. Empty when unbound. */
export function describe(b: Binding, platform = navigator.platform): string {
  if (!b.key) return 'unbound';
  const mac = /mac|iphone|ipad/i.test(platform);
  const parts: string[] = [];
  if (b.mod) parts.push(mac ? '⌘' : 'Ctrl');
  if (b.alt) parts.push(mac ? '⌥' : 'Alt');
  if (b.shift) parts.push(mac ? '⇧' : 'Shift');
  parts.push(b.key.length === 1 ? b.key.toUpperCase() : b.key);
  return parts.join(mac ? '' : '+');
}

/** Turn a keypress into a binding, for the "press a key" capture in Settings. */
export function fromEvent(e: KeyboardEvent): Binding {
  return {
    key: e.key.toLowerCase(),
    // Recorded so the binding follows the physical key rather than the character it happens to
    // produce on the layout it was captured with.
    code: e.code || undefined,
    mod: e.metaKey || e.ctrlKey,
    alt: e.altKey,
    shift: e.shiftKey,
  };
}

/** Which other command already uses this binding, if any — so a clash can be shown. */
export function conflict(
  bindings: Record<Command, Binding>,
  cmd: Command,
  b: Binding,
): Command | null {
  for (const c of Object.keys(bindings) as Command[]) {
    if (c === cmd) continue;
    const o = bindings[c];
    const sameKey = o.code && b.code ? o.code === b.code : !!o.key && o.key === b.key;
    if (sameKey && !!o.mod === !!b.mod && !!o.alt === !!b.alt && !!o.shift === !!b.shift) {
      return c;
    }
  }
  return null;
}

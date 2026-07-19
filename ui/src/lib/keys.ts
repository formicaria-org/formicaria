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
  /** `KeyboardEvent.key`, lower-cased for letters. Empty string = unbound. */
  key: string;
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

export const DEFAULTS: Record<Command, Binding> = {
  palette: { key: 'k', mod: true },
  // Bracket keys rather than Tab: the browser owns Ctrl+Tab in a real tab strip and will not
  // reliably yield it, and these are what editors have used for "cycle panel" for years.
  nextPane: { key: ']', mod: true },
  prevPane: { key: '[', mod: true },
  newNote: { key: 'c' },
  newView: { key: 'n', mod: true, shift: true },
  focusSearch: { key: '/' },
  closePane: { key: 'w', mod: true, shift: true },
  backup: { key: 'b', mod: true, shift: true },
};

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
          out[c] = { key: b.key, mod: !!b.mod, alt: !!b.alt, shift: !!b.shift };
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

/** Does this event match this binding? */
export function matches(e: KeyboardEvent, b: Binding): boolean {
  if (!b.key) return false; // explicitly unbound
  const mod = e.metaKey || e.ctrlKey;
  return (
    e.key.toLowerCase() === b.key.toLowerCase() &&
    mod === !!b.mod &&
    e.altKey === !!b.alt &&
    e.shiftKey === !!b.shift
  );
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
    if (o.key && o.key === b.key && !!o.mod === !!b.mod && !!o.alt === !!b.alt && !!o.shift === !!b.shift) {
      return c;
    }
  }
  return null;
}

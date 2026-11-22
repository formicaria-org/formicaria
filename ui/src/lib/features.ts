// **formicaria's features, declared once** (`decisions.md` 2026-11-17, *one frame*): what `+` suggests,
// what a tab's chooser (`Food ▾`) offers, what the phone's bar and the computer's new tab show. Each
// is a page of its own (`/f/…`); on the phone the same page in a tab of the app.
export interface Feature {
  /** The group or view it opens (`route.ts`). */
  key: string;
  label: string;
  /** One character, for places with no room for an icon. */
  mark: string;
  /** The icon's name (`Icon.svelte`). */
  icon: string;
  /** Its page. */
  url: string;
}

// **Read from the library's sections** (`sections.json`, written from the shipped `.kind` files by a
// test in `crates/fm-app/src/sections.rs`; the shell, P1): one list, not two. The key each feature
// had before (the diary's word for a page, a group's name) is its page's last part.
import sectionsJson from './sections.json';
import type { Section } from './types';

const keyOf = (page: string): string => page.replace(/^\/f\/(group\/)?/, '').split('/')[0];

export const FEATURES: Feature[] = (sectionsJson as Section[]).map((s) => ({
  key: keyOf(s.page),
  label: s.label,
  mark: s.mark,
  icon: s.icon,
  url: s.page,
}));

/** The feature a page address belongs to, if any. */
export function featureOf(path: string): Feature | undefined {
  return FEATURES.find((f) => path === f.url || path.startsWith(`${f.url}/`));
}

/** What a typed line means: a web address to go to, or words to search the web for. */
export function webTarget(
  typed: string,
  search: (words: string) => string = (w) => `https://duckduckgo.com/?q=${encodeURIComponent(w)}`,
): { url: string; label: string } | null {
  const t = typed.trim();
  if (!t) return null;
  if (!/\s/.test(t) && /^https?:\/\//i.test(t))
    return { url: t, label: `Go to ${t.replace(/^https?:\/\//, '')}` };
  if (!/\s/.test(t) && /^[\w-]+(\.[\w-]+)+(:\d+)?(\/.*)?$/.test(t))
    return { url: `https://${t}`, label: `Go to ${t}` };
  return {
    url: search(t),
    label: `Search the web for “${t}”`,
  };
}

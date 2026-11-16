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

export const FEATURES: Feature[] = [
  { key: 'Library', label: 'Notes', mark: '✎', icon: 'pen', url: '/f/group/Library' },
  { key: 'Tasks', label: 'Tasks', mark: '▦', icon: 'board', url: '/f/group/Tasks' },
  { key: 'agenda', label: 'Agenda', mark: '◷', icon: 'calendar', url: '/f/agenda' },
  { key: 'Food', label: 'Food journal', mark: '◍', icon: 'chart', url: '/f/group/Food' },
  { key: 'Fitness', label: 'Fitness', mark: '◆', icon: 'chart', url: '/f/group/Fitness' },
  { key: 'Health', label: 'Health', mark: '✚', icon: 'chart', url: '/f/group/Health' },
  { key: 'Music', label: 'Music', mark: '♪', icon: 'play', url: '/f/group/Music' },
];

/** The feature a page address belongs to, if any. */
export function featureOf(path: string): Feature | undefined {
  return FEATURES.find((f) => path === f.url || path.startsWith(`${f.url}/`));
}

/** What a typed line means: a web address to go to, or words to search the web for. */
export function webTarget(typed: string): { url: string; label: string } | null {
  const t = typed.trim();
  if (!t) return null;
  if (!/\s/.test(t) && /^https?:\/\//i.test(t))
    return { url: t, label: `Go to ${t.replace(/^https?:\/\//, '')}` };
  if (!/\s/.test(t) && /^[\w-]+(\.[\w-]+)+(:\d+)?(\/.*)?$/.test(t))
    return { url: `https://${t}`, label: `Go to ${t}` };
  return {
    url: `https://duckduckgo.com/?q=${encodeURIComponent(t)}`,
    label: `Search the web for “${t}”`,
  };
}

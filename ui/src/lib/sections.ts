// **The library's sections, described once** (`docs/context/workspace-design-2026-11-15.md` §11.5;
// `decisions.md` 2026-11-16, night, *the library's words*). formicaria is the library; each section
// keeps its own catalogues (views) and its own items, where they already are. This list is what the
// layer of access and linking reads: which section an item belongs to (for the library search and
// the collection's tabs), how a section opens as a tab, and what the assistant may do there.
//
// The core is untouched: a section's items are the notes (and files) they always were; a section is
// a way of finding, opening and bringing them in.

/** The fields of an item a section is recognised by (an `ObjectMeta`'s, or a search hit's). */
export interface ItemShape {
  tags?: string[];
  props?: Record<string, unknown>;
}

export interface Section {
  /** The group's key, as views name it (`group:`), unchanged so no saved view breaks. */
  key: string;
  /** What a person reads. */
  label: string;
  /** One character, for places with no room for an icon (the address bar's suggestions). */
  mark: string;
  /** The collection's tab for its catalogue: a lean tab onto formicaria at this address. */
  address: string;
  /** Does an item belong here? The first section that claims it wins; the study rooms take the rest. */
  owns: (n: ItemShape) => boolean;
  /** **What the assistant may do here** (workspace-design §11.5, D7): off until the person turns it
   *  on for this section, and then reads (search) before any write, its findings as proposals. */
  assistant: { search: boolean };
}

const has = (n: ItemShape, key: string) => n.props?.[key] != null && n.props?.[key] !== '';
const tagged = (n: ItemShape, tag: string) => n.tags?.includes(tag) ?? false;
const log = (n: ItemShape, kind: string) => n.props?.log === kind;

export const SECTIONS: Section[] = [
  {
    key: 'Mail',
    label: 'Mail',
    mark: '✉',
    address: 'view:group:Mail',
    owns: (n) => has(n, 'gmail_thread'),
    assistant: { search: false },
  },
  {
    key: 'Music',
    label: 'Music',
    mark: '♪',
    address: 'view:group:Music',
    owns: (n) => log(n, 'music') || tagged(n, 'song'),
    assistant: { search: false },
  },
  {
    key: 'Food',
    label: 'Food',
    mark: '◍',
    address: 'view:group:Food',
    owns: (n) => log(n, 'food') || has(n, 'recipe') || tagged(n, 'recipe'),
    assistant: { search: false },
  },
  {
    key: 'Fitness',
    label: 'Fitness',
    mark: '◆',
    address: 'view:group:Fitness',
    owns: (n) => log(n, 'workout'),
    assistant: { search: false },
  },
  {
    key: 'Health',
    label: 'Health',
    mark: '✚',
    address: 'view:group:Health',
    owns: (n) => log(n, 'health'),
    assistant: { search: false },
  },
  {
    key: 'Web',
    label: 'Web',
    mark: '◎',
    address: 'view:group:Library',
    owns: (n) => tagged(n, 'web') || has(n, 'source_url'),
    assistant: { search: false },
  },
  {
    key: 'Library',
    label: 'Study rooms',
    mark: '✎',
    address: 'view:group:Library',
    owns: () => true,
    assistant: { search: false },
  },
];

/** The section an item belongs to. */
export function sectionOf(n: ItemShape): Section {
  return SECTIONS.find((s) => s.owns(n)) ?? SECTIONS[SECTIONS.length - 1];
}

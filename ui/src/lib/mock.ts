// Browser-only fallback: a tiny in-memory store that answers the same commands
// the Rust backend does, so the board is developable with `pnpm dev` and no
// desktop shell. It mirrors the backend's grouping (newest-first, first-seen
// column order) so what you see in the browser matches the real window. This
// file is deliberately NOT under src/renderers — status names live here, never
// in a renderer, which is the invariant the CI grep enforces.
import { parseStamp } from './stamp';
import type {
  AssetStatus,
  BackupStatus,
  Board,
  Column,
  NoteDetail,
  ObjectMeta,
  PathCheck,
  PullResult,
  VaultInfo,
  ViewInfo,
  ViewResult,
} from './types';

/** The mock's stand-in for Rust's `Stamp::from_str`: empty clears, a valid stamp
 *  is stored verbatim (the canonical form is what the server would write back),
 *  and garbage throws the way `apply_property` returns an error. */
function parseStampOrThrow(key: string, value: string): string | null {
  if (!value) return null;
  const s = parseStamp(value);
  if (!s) throw new Error(`${key} must be YYYY-MM-DD or YYYY-MM-DDTHH:MM: ${value}`);
  return s.time ? `${s.day}T${s.time}` : s.day;
}

let seq = 0;
function makeNote(partial: Partial<ObjectMeta> & { preview: string }): ObjectMeta {
  const stamp = new Date(Date.now() - seq * 60000).toISOString();
  // `M0CK`, not `MOCK`: a ULID is Crockford base32, which excludes I, L, O and U. The old
  // prefix meant no mock id ever parsed as a ULID, so anything that checks id *shape* — the
  // discussion-message test, for one — behaved differently here than against the server.
  // Still readable as "mock", now actually well-formed.
  const id = 'M0CK' + String(seq).padStart(22, '0');
  seq += 1;
  return {
    id,
    type: 'note',
    title: null,
    status: null,
    due: null,
    start: null,
    hard: false,
    created: stamp,
    updated: stamp,
    tags: [],
    assets: [],
    props: {},
    // Derived from location in the real backend; here, just a label to badge with.
    vault: 'personal',
    ...partial,
  };
}

// Proposals accepted this session — with no git, "the branch was merged and deleted" is modelled as
// membership here, which flips `proposal_diff` to the merged/gone answer.
const acceptedProposals = new Set<string>();

const notes: ObjectMeta[] = [
  makeNote({ preview: 'GAE lambda interacts badly with inner-loop adaptation', status: 'doing', tags: ['meta-rl'], props: { project: 'alpha' } }),
  makeNote({ preview: 'Draft the trust-region clipping ablation', status: 'todo', start: '2026-07-16', due: '2026-07-20', hard: true, props: { project: 'alpha' } }),
  makeNote({ preview: 'Reply to reviewer 2', status: 'todo', due: '2026-07-11', hard: true, tags: ['neurips'], vault: 'lab' }),
  makeNote({ preview: 'Read the Muesli paper', status: 'todo', tags: ['reading'], props: { project: 'beta' } }),
  makeNote({ preview: 'Ship the second renderer', status: 'done', props: { project: 'beta' } }),
  makeNote({ preview: 'Weekly sync notes', start: '2026-07-16T14:30', due: '2026-07-16T15:00', tags: ['meeting'], vault: 'lab' }),
  makeNote({ preview: 'figure_3_final.pdf', type: 'asset', assets: ['sha256:deadbeef'], props: { project: 'alpha' } }),
  makeNote({ preview: 'poster_v2.png', type: 'asset', assets: ['sha256:cafebabe'], props: { project: 'beta' } }),
  makeNote({ preview: 'Architecture sketch', title: 'Architecture sketch', props: { view: 'board', project: 'alpha' } }),
];

// A deterministic 64-hex string from a name, so re-ingesting the same file name
// yields the same reference (mock stand-in for content-addressing).
function fakeHash(name: string): string {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h.toString(16).padStart(8, '0').repeat(8);
}

function valueOf(n: ObjectMeta, key: string): string {
  switch (key) {
    case 'type':
    case 'kind':
      return n.type;
    case 'status':
      return n.status ?? '';
    case 'title':
      return n.title ?? '';
    case 'due':
      return n.due ?? '';
    case 'start':
      return n.start ?? '';
    case 'tags':
      return n.tags.join(', ');
    default: {
      const v = n.props[key];
      return v == null ? '' : String(v);
    }
  }
}

// The board/agenda/recent commands filter to `Kind::Note` in the query layer —
// an asset is a blob a note references, not something you plan. Mirror that here
// so the mock backend answers like the real one. `search`/`gallery` still see
// assets, which is how you find one (and how the `/` menu inserts a reference).
/** A discussion message: an ordinary note carrying a well-formed `thread_of`. Mirrors
 *  `fm_app::thread::is_message` — the value must *parse as a reference*, so a note whose
 *  `thread_of` holds a stray word (a board drop, a typo) is still an ordinary note. */
const isMessage = (n: ObjectMeta) =>
  typeof n.props?.thread_of === 'string' &&
  /^note:[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/.test(n.props.thread_of as string);

/** A proposal: an ordinary note carrying a well-formed `proposes: branch:<name>`. Mirrors
 *  `fm_app::thread::is_proposal` (and `fm_model::parse_branch_ref`) — a non-empty name with no
 *  internal whitespace, surrounding whitespace tolerated — so a stray word in `proposes` leaves
 *  the note ordinary. */
const isProposal = (n: ObjectMeta) =>
  typeof n.props?.proposes === 'string' &&
  /^branch:\s*\S+\s*$/.test(n.props.proposes as string);

/** A first-class discussion: a note that is the root of its own thread (`thread_of` points at
 *  itself). Mirrors `fm_app::thread::is_discussion_root`. Being a well-formed `thread_of`, it is
 *  already a message for `isNote`, so it drops out of the planning views for free. */
const isDiscussion = (n: ObjectMeta) => n.props?.thread_of === `note:${n.id}`;

/** What the planning views show. Mirrors `fm_app::thread::notes_base()`: not an asset, not a
 *  discussion message, and not a proposal. **Keep these in step** — a mock that disagrees with
 *  the server is a mock that hides a server bug (and vice versa). */
const isNote = (n: ObjectMeta) => n.type !== 'asset' && !isMessage(n) && !isProposal(n);

function buildBoard(groupBy: string): Board {
  const sorted = notes.filter(isNote).sort((a, b) => b.created.localeCompare(a.created));
  const cols = new Map<string, Column>();
  for (const n of sorted) {
    const value = valueOf(n, groupBy);
    if (!cols.has(value)) {
      cols.set(value, { value, label: value === '' ? '(none)' : value, cards: [] });
    }
    cols.get(value)!.cards.push(n);
  }
  return { group_by: groupBy, columns: [...cols.values()] };
}

function setProp(id: string, key: string, value: string): void {
  const n = notes.find((x) => x.id === id);
  if (!n) return;
  switch (key) {
    case 'status':
      n.status = value || null;
      break;
    case 'type':
    case 'kind':
      n.type = value || 'note';
      break;
    case 'title':
      n.title = value || null;
      break;
    // Mirror `apply_property`: a stamp is `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`,
    // empty clears, and anything else is REJECTED. The mock has to refuse what
    // the Rust refuses, or the UI tests pass against a backend that doesn't exist.
    case 'due':
      n.due = parseStampOrThrow('due', value);
      break;
    case 'start':
      n.start = parseStampOrThrow('start', value);
      break;
    case 'tags':
      n.tags = value ? value.split(/[,\s]+/).filter(Boolean) : [];
      break;
    default:
      if (value) n.props[key] = value;
      else delete n.props[key];
  }
  n.updated = new Date().toISOString();
}

function captureNote(body: string, vault = ''): ObjectMeta {
  const n = makeNote({ preview: body, vault: mockVault(vault).name });
  notes.unshift(n);
  return n;
}

// A representative body for whichever note is opened, so the read view shows
// markdown + inline math + a mermaid diagram + a missing asset in browser dev.
// Exported so the render test suite can assert against the exact body that ships
// in `pnpm dev`, keeping the test tied to what a user actually sees.
export const SAMPLE_BODY = [
  '# GAE and inner-loop adaptation',
  '',
  'The GAE lambda interacts badly with inner-loop adaptation. With $\\lambda = 0.95$',
  'the advantage estimate leaks across the meta-update boundary:',
  '',
  '$$A_t = \\sum_{l=0}^{\\infty} (\\gamma\\lambda)^l \\delta_{t+l}$$',
  '',
  '```mermaid',
  'graph LR; sample --> inner_loop --> meta_update --> sample',
  '```',
  '',
  '![trust-region figure](asset:sha256-deadbeef)',
  '',
  // A reference to another note, alongside the asset reference above — the two
  // are deliberately the same shape. `MOCK…0001` is the clipping-ablation note.
  'Follow-up on [the clipping ablation](note:M0CK0000000000000000000001).',
  '',
  '- pin `unicode61 remove_diacritics 2`',
  '- measure the worst case',
].join('\n');

// A blank Excalidraw scene — what a board note's body looks like before anything
// is drawn (the real backend stores the same shape).
const EMPTY_BOARD =
  '{"type":"excalidraw","version":2,"source":"formicaria","elements":[],"appState":{},"files":{}}';

const bodyOverrides = new Map<string, string>();

/// A cheap stand-in for the server's sha256 body version. It only has to be *a function of
/// the body* — the mock is the only thing that ever compares it with itself — so it says what
/// it is rather than pretending to be sha256.
function mockVersion(body: string): string {
  let h = 0;
  for (let i = 0; i < body.length; i++) h = (Math.imul(31, h) + body.charCodeAt(i)) | 0;
  return `mock-${(h >>> 0).toString(16)}`;
}

function noteDetail(id: string): NoteDetail | null {
  const n = notes.find((x) => x.id === id);
  if (!n) return null;
  const body =
    bodyOverrides.get(id) ??
    (n.props.view === 'board'
      ? EMPTY_BOARD
      : n.type === 'asset'
        ? `# ${n.title ?? n.preview}\n\n${n.preview}`
        : SAMPLE_BODY);
  return { ...n, body, version: mockVersion(body) };
}

// Backing up is inert here — there is no vault to push — but the remote is
// remembered so the setup flow stays exercisable under `pnpm dev`: save a URL and
// watch the panel change what it promises.
// Two vaults, so `pnpm dev` exercises the plural: badges, the filter, and a backup
// panel that is a list rather than a form. The first is the default, as in the real
// config. Identity starts null on both, like a machine with no git config, so the
// identity question is on screen rather than on the path only configured users see.
const gitVaults: Array<{
  name: string;
  remote: string | null;
  identity: { name: string; email: string } | null;
}> = [
  { name: 'personal', remote: null, identity: null },
  { name: 'lab', remote: null, identity: null },
];

/** The vaults `list_vaults` reports. Mutable: `create_vault` appends, so the first-run
 *  screen's success path is developable without a backend. */
// `git_assets_max` is mutable here because `set_git_assets_max` writes it — the mock models the
// setting round-tripping, which is the only way the Settings field can be developed against it.
// `personal` starts off (the real default) and `lab` starts on, so both states are on screen.
const mockVaults: Array<{ name: string; path: string; git_assets_max: number | null }> = [
  { name: 'personal', path: '/home/you/notes', git_assets_max: null },
  { name: 'lab', path: '/home/you/lab-notes', git_assets_max: 2_000_000 },
];

/** The size grammar the Rust accepts, mirrored so the mock refuses what the backend refuses. */
function parseSize(text: string): number | null {
  const t = text.trim().toLowerCase();
  if (!t || t === 'off' || t === 'none') return null;
  const m = /^([0-9]*\.?[0-9]+)\s*(b|kb|mb|gb|kib|mib|gib)?$/.exec(t);
  if (!m) throw new Error(`${JSON.stringify(text)} is not a size — try 2MB, 500kB, or leave it empty for none`);
  const mult: Record<string, number> = {
    '': 1, b: 1, kb: 1e3, mb: 1e6, gb: 1e9,
    kib: 1024, mib: 1024 ** 2, gib: 1024 ** 3,
  };
  return Math.round(parseFloat(m[1]) * mult[m[2] ?? '']);
}

/** Resolve a vault by name; empty means the default. Unknown throws, exactly as the
 *  real backend refuses — a typo must not quietly write into another audience. */
function mockVault(name: unknown): (typeof gitVaults)[number] {
  const n = String(name ?? '').trim();
  if (!n) return gitVaults[0];
  const v = gitVaults.find((x) => x.name === n);
  if (!v) throw new Error(`no vault named '${n}'`);
  return v;
}

/**
 * The in-memory backend for `pnpm dev` and Vitest.
 *
 * **Every arm binds its value to the real DTO type before the `as T`.** The cast itself is
 * forced by the generic signature and cannot go away — but it is exactly what let this file
 * drift: it kept a top-level `restic_repo` long after restic became per-vault, and `tsc` said
 * nothing, so UI tests passed against a shape the Rust never sends. Binding first means a
 * mock that answers the wrong shape fails `check-ui` rather than a user.
 */
// The study-assistant on/off setting, mocked (the real one is a per-device launcher setting served
// by fm-serve, not a vault command).
let mockAgentEnabled = false;

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case 'agent_status':
      return { enabled: mockAgentEnabled } as T;
    case 'set_agent':
      mockAgentEnabled = Boolean(args.enabled);
      return { ok: true } as T;
    case 'agent_activity_poll':
      return { active: false } as T;
    case 'agents':
      // In the mock, the "enabled" toggle stands in for a live agent, so the @-picker is demoable.
      return { agents: mockAgentEnabled ? ['lfm2.5-230m'] : [] } as T;
    case 'board': {
      const board: Board = buildBoard(String(args.groupBy));
      return board as T;
    }
    case 'gallery': {
      const assets: ObjectMeta[] = [...notes]
        .filter((n) => n.type === 'asset')
        .sort((a, b) => b.created.localeCompare(a.created));
      return assets as T;
    }
    case 'agenda': {
      const dated: ObjectMeta[] = notes
        .filter((n) => isNote(n) && n.due && n.status !== 'done')
        .sort((a, b) => (a.due ?? '').localeCompare(b.due ?? ''));
      return dated as T;
    }
    case 'get': {
      const detail: NoteDetail | null = noteDetail(String(args.id));
      return detail as T;
    }
    case 'capture': {
      const meta: ObjectMeta = captureNote(String(args.body), String(args.vault ?? ''));
      return meta as T;
    }
    case 'set_property': {
      const key = String(args.key);
      // Mirror the server's refusal: discussion structure is written by `reply`, never by
      // hand — which is what stops a board grouped by `thread_of` erasing a note on one drag.
      if (key === 'thread_of' || key === 'reply_to') {
        throw new Error(`\`${key}\` is discussion structure — reply to a note instead`);
      }
      setProp(String(args.id), key, String(args.value));
      return undefined as T;
    }
    // A first-class discussion — a note that is the root of its own thread. Created explicitly
    // because the self-anchor `thread_of` is refused by `set_property` (mirrors the server).
    case 'create_discussion': {
      const title = String(args.title ?? '').trim();
      if (!title) throw new Error('a discussion needs a title');
      const d = makeNote({ preview: '', title, vault: String(args.vault || 'personal') });
      d.props = { thread_of: `note:${d.id}` };
      notes.unshift(d);
      return d as T;
    }
    // Every first-class discussion, most-recently-active first, with a message count and fake
    // participants (the server reads these from git; the mock invents them like `activity` does).
    case 'discussions': {
      const people = ['Ada Lovelace', 'Ravi Kumar'];
      const list = notes
        .filter(isDiscussion)
        .map((r) => {
          const msgs = notes.filter((n) => n.props?.thread_of === `note:${r.id}` && n.id !== r.id);
          const names = [
            ...new Set(msgs.map((m) => people[Number(m.id.replace(/\D/g, '')) % people.length])),
          ];
          const participants = names.map((name) => ({
            name,
            email: `${name.split(' ')[0].toLowerCase()}@example.org`,
          }));
          const last = msgs.reduce((acc, m) => (m.created > acc ? m.created : acc), r.created);
          return { ...r, count: msgs.length, last_activity: last, participants };
        })
        .sort((a, b) => b.last_activity.localeCompare(a.last_activity));
      return list as T;
    }
    // "What links here" — scan bodies for `note:<target>`, mirroring the server's ref scan.
    case 'backlinks': {
      const target = String(args.id);
      const list = notes
        .filter(isNote)
        .filter((n) => n.id !== target)
        .filter((n) => (bodyOverrides.get(n.id) ?? n.preview).includes(`note:${target}`))
        .sort((a, b) => b.updated.localeCompare(a.updated));
      return list as T;
    }
    case 'conflicts': {
      // A note is in conflict iff its body carries both markers — same rule as the server.
      const marked = (s: string) => /^<{7}/m.test(s) && /^>{7}/m.test(s);
      return notes
        .filter(isNote)
        .filter((n) => marked(bodyOverrides.get(n.id) ?? n.preview))
        .sort((a, b) => b.updated.localeCompare(a.updated)) as T;
    }
    case 'templates': {
      // A template is just a note tagged `template`, exactly as the server sees it (a `TagsAll`
      // filter over the plain note set) — no hidden class, most-recently-touched first.
      const list = notes
        .filter(isNote)
        .filter((n) => n.tags.includes('template'))
        .sort((a, b) => b.updated.localeCompare(a.updated));
      return list as T;
    }
    case 'reply': {
      const target = notes.find((n) => n.id === String(args.id));
      if (!target) throw new Error('no such note');
      const body = String(args.body);
      if (!body.trim()) throw new Error('a message needs something in it');
      // Re-root exactly as the server does: replying to a message joins that message's
      // discussion rather than starting one hanging off a note no view can reach.
      const rootRef =
        (isMessage(target) && (target.props.thread_of as string)) || `note:${target.id}`;
      const m = makeNote({ preview: body, vault: target.vault });
      m.props = { thread_of: rootRef, reply_to: `note:${target.id}` };
      notes.unshift(m);
      return m as T;
    }
    case 'proposal_diff': {
      const prop = notes.find((n) => n.id === String(args.id));
      if (!prop || !isProposal(prop)) throw new Error('not a proposal');
      // Once accepted, the branch is gone — the proposal *note* outlives it, so the diff reports
      // "nothing to show", exactly as the real backend does after a merge.
      if (acceptedProposals.has(prop.id)) return { exists: false, files: [], patch: '' } as T;
      // The mock has no git, so it returns a stand-in patch (there is no real branch to diff). The
      // real backend diffs `proposal/<id>` against `main`. Shape matches the server's `ProposalDiff`.
      return {
        exists: true,
        files: ['notes/(preview).md'],
        patch: '@@ preview @@\n+ (the real diff appears against a live backend)\n',
      } as T;
    }
    case 'accept_proposal': {
      // No git in the mock, so "merging the branch" is modelled by marking the proposal accepted: its
      // note stays (immortal), but its branch is now gone, so `proposal_diff` reports it done. The real
      // backend merges `proposal/<id>` into main and deletes the branch.
      const prop = notes.find((n) => n.id === String(args.id) && isProposal(n));
      if (!prop) return { outcome: 'already_gone' } as T;
      acceptedProposals.add(prop.id);
      return { outcome: 'merged' } as T;
    }
    case 'create_proposal': {
      const target = notes.find((n) => n.id === String(args.id));
      if (!target) throw new Error('no such note');
      // The mock has no git, so it records the proposal *note* the Collaboration feed lists; the
      // real backend also builds the `proposal/<id>` branch and enforces the vault's size guardrails.
      const title = target.title ?? 'note';
      const p = makeNote({ preview: `Proposed change to ${title}`, title: `Proposal: ${title}`, vault: target.vault });
      p.props = { proposes: `branch:proposal/${p.id}` };
      notes.unshift(p);
      return p as T;
    }
    case 'thread': {
      const rootId = String(args.id);
      const ref = `note:${rootId}`;
      // Sorted by **id**, not by `created`. On the server those are the same order, because a
      // ULID is time-sortable and that is exactly why messages are ULID-named. Here they are
      // not: `makeNote` back-dates each fixture note so the seeded list looks like a timeline,
      // which would read a thread backwards. Sorting by id reproduces the server's semantics
      // rather than the shape of its code.
      const messages = notes
        // Exclude the root itself: a first-class discussion is self-anchored, so it would
        // otherwise appear as the first message in its own thread (mirrors the server).
        .filter((n) => n.props?.thread_of === ref && n.id !== rootId)
        .sort((a, b) => a.id.localeCompare(b.id));
      const at = new Map(messages.map((m, i) => [m.id, i]));
      return {
        root: notes.find((n) => n.id === rootId) ?? null,
        count: messages.length,
        messages: messages.map((m) => {
          // Depth by walking parents, cycle-guarded and capped — the same shape as the
          // server, so the mock cannot make an indent the real backend would not produce.
          let depth = 0;
          const seen = new Set<string>();
          let cur = m;
          for (;;) {
            const parent = String(cur.props?.reply_to ?? '').replace('note:', '');
            const i = at.get(parent);
            if (i === undefined || seen.has(cur.id) || depth >= 4) break;
            seen.add(cur.id);
            depth += 1;
            cur = messages[i];
          }
          return {
            ...m,
            // A message's body IS its text — short, and written straight into the preview.
            body: bodyOverrides.get(m.id) ?? m.preview,
            reply_to: String(m.props?.reply_to ?? '').replace('note:', '') || null,
            depth,
          };
        }),
      } as T;
    }
    // Derived from git on the server; here, simply the oldest notes, so the panel has data.
    case 'stale': {
      return notes
        .filter(isNote)
        .slice()
        .sort((a, b) => a.updated.localeCompare(b.updated))
        .slice(0, 5) as T;
    }
    case 'update_body': {
      const id = String(args.id);
      const body = String(args.body);
      const base = String(args.base ?? '');
      const n = notes.find((x) => x.id === id);
      // Mirror the real guard, so the mock cannot quietly accept a write the server would
      // refuse — that divergence is exactly what `known-issues.md` warns the mock does.
      // Compared against the *current* body's version, as the server does.
      const current = noteDetail(id);
      if (n && base && current && mockVersion(current.body) !== base) {
        throw new Error('this note changed on disk since you opened it — reload before saving');
      }
      bodyOverrides.set(id, body);
      if (n) {
        n.updated = new Date().toISOString();
        // Mirror the real backend: a plain note's card preview is derived from its
        // body (first non-empty line, minus a leading Markdown heading marker). A
        // board's body is scene JSON — never surface that, it keeps its title.
        if (n.props.view !== 'board') {
          const firstLine = body.split('\n').map((l) => l.trim()).find((l) => l) ?? '';
          n.preview = firstLine.replace(/^#+\s+/, '') || n.preview;
        }
      }
      return mockVersion(body) as T;
    }
    case 'delete': {
      const id = String(args.id);
      const i = notes.findIndex((x) => x.id === id);
      if (i >= 0) notes.splice(i, 1);
      bodyOverrides.delete(id);
      return undefined as T;
    }
    case 'copy_note': {
      const src = notes.find((x) => x.id === String(args.id));
      if (!src) throw new Error('note not found');
      const target = mockVault(args.vault).name;
      if (src.vault === target) throw new Error(`note is already in vault '${target}'`);
      const withAssets = Boolean(args.with_assets);
      // Mock provenance: the raw source id is a fine match key here (the real backend hashes it).
      const token = src.id;
      // Override, don't duplicate: drop any prior copy of this source in the target.
      let replaced = 0;
      for (let i = notes.length - 1; i >= 0; i--) {
        if (notes[i].vault === target && notes[i].props.copy_of === token) {
          notes.splice(i, 1);
          replaced++;
        }
      }
      // A copy is a new note (fresh id from makeNote) in the target vault; drop the source id.
      const { id: _drop, ...rest } = src;
      const copy = makeNote({ ...rest, vault: target });
      copy.assets = withAssets ? [...src.assets] : []; // prose-only strips attachments
      copy.props = { ...copy.props, copy_of: token };
      notes.unshift(copy);
      const new_blobs = withAssets ? src.assets.map((a) => a.replace(/^sha256:/, '')) : [];
      return { meta: copy, new_blobs, replaced } as T;
    }
    case 'copy_status': {
      const src = notes.find((x) => x.id === String(args.id));
      const target = mockVault(args.vault).name;
      return notes.some((n) => n.vault === target && n.props.copy_of === src?.id) as T;
    }
    case 'uncopy_note': {
      const i = notes.findIndex((x) => x.id === String(args.id));
      if (i >= 0) notes.splice(i, 1);
      return undefined as T;
    }
    case 'search': {
      const q = String(args.query ?? '').trim().toLowerCase();
      if (!q) return [] as T;
      return notes
        .filter((n) =>
          [n.preview, n.title ?? '', n.tags.join(' ')].join(' ').toLowerCase().includes(q),
        )
        .sort((a, b) => b.updated.localeCompare(a.updated)) as T;
    }
    case 'recent': {
      const recent: ObjectMeta[] = notes
        .filter(isNote)
        .sort((a, b) => b.created.localeCompare(a.created));
      return recent as T;
    }
    // The Collaboration surface's feed: proposals only — the complement of the exclusion above,
    // newest-first, exactly what `commands::proposals` returns.
    case 'proposals': {
      const list: ObjectMeta[] = notes
        .filter(isProposal)
        .sort((a, b) => b.created.localeCompare(a.created));
      return list as T;
    }
    // Fake git authorship for dev/tests: attribute each note to one of two people, newest-first,
    // so the "edited by" labels, the activity stream, and the contributor filter all have data.
    case 'activity': {
      const people = ['Ada Lovelace', 'Ravi Kumar'];
      // Messages are excluded here too. The server does it by hand because `activity` is a
      // `git log` read-model with no filter to hang a predicate on; the mock mirrors that
      // rather than the shape of the code.
      return notes
        .filter(isNote)
        .map((n, i) => ({
          id: n.id,
          title: n.title ?? n.preview,
          type: n.type,
          vault: n.vault,
          author: people[i % people.length],
          email: `${people[i % people.length].split(' ')[0].toLowerCase()}@example.com`,
          time: n.updated,
        }))
        .sort((a, b) => b.time.localeCompare(a.time)) as T;
    }
    case 'ingest': {
      // No vault in the browser/test: synthesize an asset note so the editor can
      // insert a reference. Deterministic hash so re-adding the same name "dedups".
      const name = String(args.name ?? 'asset');
      // The asset joins the audience of the note it was dropped on — same rule the real
      // backend follows, mirrored so `pnpm dev` can't show a flow the backend refuses.
      const vault = mockVault(args.vault).name;
      const n = makeNote({
        preview: name,
        type: 'asset',
        title: name,
        assets: [`sha256:${fakeHash(name)}`],
        vault,
      });
      notes.unshift(n);
      return n as T;
    }
    // No vault in the browser/test: assets can't be resolved (callers fall back
    // to the missing placeholder), and durability commands are inert no-ops.
    case 'resolve_asset':
      return null as T;
    case 'asset_status': {
      const status: AssetStatus = { has_blob: false, has_thumb: false, mime: null };
      return status as T;
    }
    case 'open_external':
      return undefined as T;
    // The vault list the mock models. Two, so the sidebar's vault chips and filter are
    // exercised (they only render above one). The first-run state — an empty list — is
    // reached for real, not here: `pnpm dev` should give you a working app.
    case 'list_vaults': {
      const vaults: VaultInfo[] = mockVaults.map((v, i) => ({ ...v, default: i === 0 }));
      return vaults as T;
    }
    case 'set_git_assets_max': {
      const target = String(args.vault ?? '');
      const v = mockVaults.find((x) => x.name === target) ?? mockVaults[0];
      v.git_assets_max = parseSize(String(args.max ?? ''));
      const vaults: VaultInfo[] = mockVaults.map((x, i) => ({ ...x, default: i === 0 }));
      return vaults as T;
    }
    case 'check_path': {
      // Mirrors the real policy closely enough to develop the form against, and no
      // further: the server owns `ok`, and this file must never become a second opinion.
      const name = String(args.name ?? '');
      const path = String(args.path ?? '');
      const name_ok = name.trim().length > 0;
      const name_taken = mockVaults.some((v) => v.name === name);
      const path_taken = mockVaults.some((v) => v.path === path);
      const check: PathCheck = {
        path,
        exists: false,
        empty: false,
        notes: 0,
        not_a_directory: false,
        parent_missing: false,
        writable: true,
        git_repo: false,
        name_ok,
        name_taken,
        path_taken,
        overlaps: null,
        config_writable: true,
        ok: name_ok && !name_taken && !path_taken && path.trim().length > 0,
      };
      return check satisfies PathCheck as T;
    }
    case 'config':
      // Shaped like the real thing, including the awkward parts — a null vault_list and an
      // unwritable one are exactly the states the panel must render honestly, and a mock that
      // only ever returns the happy case is how those go untested.
      return {
        vault_list: '~/.config/formicaria/vaults.json',
        vault_list_writable: true,
        vaults: mockVaults.map((v, i) => ({ ...v, default: i === 0 })),
        restic: mockVaults.map((v) => ({ vault: v.name, repo: null })),
        env: [{ name: 'FM_VAULT', value: 'vault' }],
        git: true,
        // Installed but not unlocked — the state that exercises the distinction between the
        // three restic questions rather than collapsing them into one happy case.
        restic_installed: true,
        restic_password_set: false,
        // The desktop shape: the user picks their own locations. The managed-root path is a
        // phone, and is exercised there.
        vault_root: null,
        ca_bundle: null, // the desktop shape: the system store is used, none is built
      } as T;
    case 'create_vault': {
      mockVaults.push({
        name: String(args.name ?? ''),
        path: String(args.path ?? ''),
        // A new or acquired vault has no opinion yet, which means notes only — the real default.
        git_assets_max: null,
      });
      const created: VaultInfo[] = mockVaults.map((v, i) => ({ ...v, default: i === 0 }));
      return created as T;
    }
    case 'probe_remote': {
      // Shaped like the real thing, including the states the form must render honestly. The
      // mock models a desktop with a plaintext helper — which is a real and common setup, and
      // the one whose advice is easiest to get wrong.
      const url = String(args.url ?? '');
      const state = !url.trim()
        ? 'unreachable'
        : url.includes('private')
          ? 'needs_auth'
          : url.includes('nope')
            ? 'unreachable'
            : 'reachable';
      return {
        state,
        detail:
          state === 'reachable'
            ? 'This repo answered — you can clone it.'
            : state === 'needs_auth'
              ? 'This repo needs credentials.'
              : url.trim()
                ? `fatal: repository '${url}' not found`
                : '',
        helper: 'store',
        helper_is_plaintext: true,
      } as T;
    }
    case 'git_auth':
      return {
        storage: 'system',
        have_credential: false,
        helper: { configured: 'store', plaintext: true, better: 'libsecret' },
      } as T;
    case 'set_git_credential': {
      if (!String(args.token ?? '').trim()) throw new Error('paste a token');
      return {
        storage: 'system',
        have_credential: true,
        helper: { configured: 'store', plaintext: true, better: 'libsecret' },
      } as T;
    }
    case 'clear_git_credential':
      return {
        storage: 'system',
        have_credential: false,
        helper: { configured: 'store', plaintext: true, better: 'libsecret' },
      } as T;
    case 'clone_vault': {
      // Registers exactly like `create_vault` — the clone itself is git, which the mock does
      // not model. It *does* enforce the identity, because that rule is the point of the
      // command and a mock that skipped it would let the form ship a state the server refuses.
      if (!String(args.url ?? '').trim()) throw new Error('a shared vault needs the URL of the repo to clone');
      if (!String(args.gitName ?? '').trim() || !String(args.gitEmail ?? '').trim())
        throw new Error('a shared vault needs your name and email');
      if (!String(args.gitEmail ?? '').includes('@'))
        throw new Error(`'${args.gitEmail}' is not an email address`);
      mockVaults.push({
        name: String(args.name ?? ''),
        path: String(args.path ?? ''),
        // A new or acquired vault has no opinion yet, which means notes only — the real default.
        git_assets_max: null,
      });
      const cloned: VaultInfo[] = mockVaults.map((v, i) => ({ ...v, default: i === 0 }));
      return cloned as T;
    }
    case 'restore_vault': {
      // Registers like the other two. The restore itself is restic, which the mock does not
      // model — but the refusals *are* modelled, because they are what the form must not be
      // able to walk past. No identity is demanded here, and that asymmetry with
      // `clone_vault` is deliberate: a clone has an audience, a restore has one user.
      if (!String(args.repo ?? '').trim())
        throw new Error('restoring needs the restic repository the backup is in');
      mockVaults.push({
        name: String(args.name ?? ''),
        path: String(args.path ?? ''),
        // A new or acquired vault has no opinion yet, which means notes only — the real default.
        git_assets_max: null,
      });
      const restored: VaultInfo[] = mockVaults.map((v, i) => ({ ...v, default: i === 0 }));
      return restored as T;
    }
    // Saved views. The mock ships one so the sidebar's view list is exercised; a real
    // `.view` lives in the vault and is parsed server-side, which the mock does not model.
    case 'list_views': {
      const views: ViewInfo[] = [{ name: 'Recent notes', renderer: 'timeline', group_by: null }];
      return views as T;
    }
    case 'run_view': {
      // Reuse the mock's own note set; the sample view just lists them like the timeline.
      const rows = notes.filter(isNote).sort((a, b) => b.created.localeCompare(a.created));
      const result: ViewResult = {
        name: String(args.name ?? ''),
        renderer: 'timeline',
        group_by: null,
        rows,
      };
      return result as T;
    }
    case 'ping':
      // Nothing writes this vault but us, so it never moves under the app. `git: true`
      // because the mock models a working machine; the no-git path is exercised for real.
      return { changed: false, git: true, restic: true, skipped: [] } as T;
    case 'open_skipped':
    case 'read_skipped':
    case 'resolve_skipped':
      // Nothing here is ever unreadable, so these are only reachable from a hand-crafted
      // call. Fail the way the real arms do rather than pretending they worked.
      throw new Error('not a currently-unreadable note');
    case 'commit':
      // Nothing to commit, and nothing blocking it — the mock vault is never mid-merge.
      return { committed: false, conflicts: [] } as T;
    case 'backup':
      return undefined as T;
    case 'backup_status': {
      // `satisfies` and not a bare `as T`: this file's whole job is to answer exactly
      // what the Rust answers, and a plain cast lets it drift silently — which it had,
      // still carrying a top-level restic long after restic became per vault.
      const status: BackupStatus = {
        vaults: gitVaults.map((v) => ({
          name: v.name,
          remote: v.remote,
          unpushed: v.remote ? 2 : null,
          identity: v.identity,
          remote_moved: v.remote ? false : null,
          conflicts: [],
          // Per vault, like the real backend: a restic repo is per repository. Null here
          // because the mock has no vault to snapshot, which also puts the "this vault's
          // media has nowhere to go" wording on screen under `pnpm dev`.
          restic_repo: null,
          restic_ready: false,
        })),
        git: true,
        restic: true,
      };
      return status as T;
    }
    case 'set_git_remote': {
      const v = mockVault(args.vault);
      const name = String(args.name ?? '').trim();
      const email = String(args.email ?? '').trim();
      if (name && email) v.identity = { name, email };
      // The same rule fm-core enforces: a vault gains an audience only once
      // someone real owns it. Mirrored here so the mock cannot drift into
      // promising a flow the real backend refuses.
      if (!v.identity) {
        throw new Error('tell us who you are first — your name and email sign every commit you share');
      }
      v.remote = String(args.url ?? '').trim() || null;
      return undefined as T;
    }
    case 'push':
      return 2 as T;
    case 'pull':
      // Nothing to pull from: there is no vault and no remote here.
      return { merged: 0, conflicts: [] } satisfies PullResult as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

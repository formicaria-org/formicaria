// Browser-only fallback: a tiny in-memory store that answers the same commands
// the Rust backend does, so the board is developable with `pnpm dev` and no
// desktop shell. It mirrors the backend's grouping (newest-first, first-seen
// column order) so what you see in the browser matches the real window. This
// file is deliberately NOT under src/renderers — status names live here, never
// in a renderer, which is the invariant the CI grep enforces.
import { parseStamp } from './stamp';
import type { Board, Column, ObjectMeta } from './types';

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
  const id = 'MOCK' + String(seq).padStart(22, '0');
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
    ...partial,
  };
}

const notes: ObjectMeta[] = [
  makeNote({ preview: 'GAE lambda interacts badly with inner-loop adaptation', status: 'doing', tags: ['meta-rl'], props: { project: 'alpha' } }),
  makeNote({ preview: 'Draft the trust-region clipping ablation', status: 'todo', start: '2026-07-16', due: '2026-07-20', hard: true, props: { project: 'alpha' } }),
  makeNote({ preview: 'Reply to reviewer 2', status: 'todo', due: '2026-07-11', hard: true, tags: ['neurips'] }),
  makeNote({ preview: 'Read the Muesli paper', status: 'todo', tags: ['reading'], props: { project: 'beta' } }),
  makeNote({ preview: 'Ship the second renderer', status: 'done', props: { project: 'beta' } }),
  makeNote({ preview: 'Weekly sync notes', start: '2026-07-16T14:30', due: '2026-07-16T15:00', tags: ['meeting'] }),
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
const isNote = (n: ObjectMeta) => n.type !== 'asset';

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

function captureNote(body: string): ObjectMeta {
  const n = makeNote({ preview: body });
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
  'Follow-up on [the clipping ablation](note:MOCK0000000000000000000001).',
  '',
  '- pin `unicode61 remove_diacritics 2`',
  '- measure the worst case',
].join('\n');

// A blank Excalidraw scene — what a board note's body looks like before anything
// is drawn (the real backend stores the same shape).
const EMPTY_BOARD =
  '{"type":"excalidraw","version":2,"source":"formicarium","elements":[],"appState":{},"files":{}}';

const bodyOverrides = new Map<string, string>();

function noteDetail(id: string): (ObjectMeta & { body: string }) | null {
  const n = notes.find((x) => x.id === id);
  if (!n) return null;
  const body =
    bodyOverrides.get(id) ??
    (n.props.view === 'board'
      ? EMPTY_BOARD
      : n.type === 'asset'
        ? `# ${n.title ?? n.preview}\n\n${n.preview}`
        : SAMPLE_BODY);
  return { ...n, body };
}

// Backing up is inert here — there is no vault to push — but the remote is
// remembered so the setup flow stays exercisable under `pnpm dev`: save a URL and
// watch the panel change what it promises.
let gitRemote: string | null = null;
// Starts null, like a vault on a machine with no git config, so `pnpm dev` shows
// the identity question rather than the path only configured users ever see.
let gitIdentity: { name: string; email: string } | null = null;

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case 'board':
      return buildBoard(String(args.groupBy)) as T;
    case 'gallery':
      return [...notes]
        .filter((n) => n.type === 'asset')
        .sort((a, b) => b.created.localeCompare(a.created)) as T;
    case 'agenda':
      return notes
        .filter((n) => isNote(n) && n.due && n.status !== 'done')
        .sort((a, b) => (a.due ?? '').localeCompare(b.due ?? '')) as T;
    case 'get':
      return noteDetail(String(args.id)) as T;
    case 'capture':
      return captureNote(String(args.body)) as T;
    case 'set_property':
      setProp(String(args.id), String(args.key), String(args.value));
      return undefined as T;
    case 'update_body': {
      const id = String(args.id);
      const body = String(args.body);
      bodyOverrides.set(id, body);
      const n = notes.find((x) => x.id === id);
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
      return undefined as T;
    }
    case 'delete': {
      const id = String(args.id);
      const i = notes.findIndex((x) => x.id === id);
      if (i >= 0) notes.splice(i, 1);
      bodyOverrides.delete(id);
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
    case 'recent':
      return notes.filter(isNote).sort((a, b) => b.created.localeCompare(a.created)) as T;
    case 'ingest': {
      // No vault in the browser/test: synthesize an asset note so the editor can
      // insert a reference. Deterministic hash so re-adding the same name "dedups".
      const name = String(args.name ?? 'asset');
      const n = makeNote({ preview: name, type: 'asset', title: name, assets: [`sha256:${fakeHash(name)}`] });
      notes.unshift(n);
      return n as T;
    }
    // No vault in the browser/test: assets can't be resolved (callers fall back
    // to the missing placeholder), and durability commands are inert no-ops.
    case 'resolve_asset':
      return null as T;
    case 'asset_status':
      return { has_blob: false, has_thumb: false, mime: null } as T;
    case 'open_external':
      return undefined as T;
    case 'ping':
      // Nothing writes this vault but us, so it never moves under the app.
      return { changed: false } as T;
    case 'commit':
      return false as T;
    case 'backup':
      return undefined as T;
    case 'backup_status':
      return {
        remote: gitRemote,
        unpushed: gitRemote ? 2 : null,
        restic_repo: null,
        restic_ready: false,
        identity: gitIdentity,
        remote_moved: gitRemote ? false : null,
        conflicts: [],
      } as T;
    case 'set_git_remote': {
      const name = String(args.name ?? '').trim();
      const email = String(args.email ?? '').trim();
      if (name && email) gitIdentity = { name, email };
      // The same rule fm-core enforces: a vault gains an audience only once
      // someone real owns it. Mirrored here so the mock cannot drift into
      // promising a flow the real backend refuses.
      if (!gitIdentity) {
        throw new Error('tell us who you are first — your name and email sign every commit you share');
      }
      gitRemote = String(args.url ?? '').trim() || null;
      return undefined as T;
    }
    case 'push':
      return 2 as T;
    case 'pull':
      // Nothing to pull from: there is no vault and no remote here.
      return { merged: 0, conflicts: [] } as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

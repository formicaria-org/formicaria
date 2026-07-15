// Browser-only fallback: a tiny in-memory store that answers the same commands
// the Rust backend does, so the board is developable with `pnpm dev` and no
// desktop shell. It mirrors the backend's grouping (newest-first, first-seen
// column order) so what you see in the browser matches the real window. This
// file is deliberately NOT under src/renderers — status names live here, never
// in a renderer, which is the invariant the CI grep enforces.
import type { Board, Column, ObjectMeta } from './types';

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
  makeNote({ preview: 'Weekly sync notes', due: '2026-07-16', tags: ['meeting'] }),
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

function buildBoard(groupBy: string): Board {
  const sorted = [...notes].sort((a, b) => b.created.localeCompare(a.created));
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
    case 'due':
      n.due = value || null;
      break;
    case 'start':
      n.start = value || null;
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

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case 'board':
      return buildBoard(String(args.groupBy)) as T;
    case 'gallery':
      return [...notes]
        .filter((n) => n.type === 'asset')
        .sort((a, b) => b.created.localeCompare(a.created)) as T;
    case 'agenda':
      return [...notes]
        .filter((n) => n.due && n.status !== 'done')
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
      bodyOverrides.set(id, String(args.body));
      const n = notes.find((x) => x.id === id);
      if (n) n.updated = new Date().toISOString();
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
      return [...notes].sort((a, b) => b.created.localeCompare(a.created)) as T;
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
      return undefined as T;
    case 'commit':
      return false as T;
    case 'backup':
      return undefined as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

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
    hard: false,
    created: stamp,
    updated: stamp,
    tags: [],
    props: {},
    ...partial,
  };
}

const notes: ObjectMeta[] = [
  makeNote({ preview: 'GAE lambda interacts badly with inner-loop adaptation', status: 'doing', tags: ['meta-rl'], props: { project: 'alpha' } }),
  makeNote({ preview: 'Draft the trust-region clipping ablation', status: 'todo', due: '2026-07-20', hard: true, props: { project: 'alpha' } }),
  makeNote({ preview: 'Read the Muesli paper', status: 'todo', tags: ['reading'], props: { project: 'beta' } }),
  makeNote({ preview: 'Ship the second renderer', status: 'done', props: { project: 'beta' } }),
  makeNote({ preview: 'Weekly sync notes', type: 'meeting', due: '2026-07-16' }),
  makeNote({ preview: 'figure_3_final.pdf', type: 'asset', props: { project: 'alpha' } }),
  makeNote({ preview: 'poster_v2.png', type: 'asset', props: { project: 'beta' } }),
];

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

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case 'board':
      return buildBoard(String(args.groupBy)) as T;
    case 'gallery':
      return [...notes]
        .filter((n) => n.type === 'asset')
        .sort((a, b) => b.created.localeCompare(a.created)) as T;
    case 'capture':
      return captureNote(String(args.body)) as T;
    case 'set_property':
      setProp(String(args.id), String(args.key), String(args.value));
      return undefined as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

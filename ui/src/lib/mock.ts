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
    // Derived from location in the real backend; here, just a label to badge with.
    vault: 'personal',
    ...partial,
  };
}

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
  'Follow-up on [the clipping ablation](note:MOCK0000000000000000000001).',
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
const mockVaults: Array<{ name: string; path: string }> = [
  { name: 'personal', path: '/home/you/notes' },
  { name: 'lab', path: '/home/you/lab-notes' },
];

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
export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
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
    case 'set_property':
      setProp(String(args.id), String(args.key), String(args.value));
      return undefined as T;
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
    // Fake git authorship for dev/tests: attribute each note to one of two people, newest-first,
    // so the "edited by" labels, the activity stream, and the contributor filter all have data.
    case 'activity': {
      const people = ['Ada Lovelace', 'Ravi Kumar'];
      return notes
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
      } as T;
    case 'create_vault': {
      mockVaults.push({ name: String(args.name ?? ''), path: String(args.path ?? '') });
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
      mockVaults.push({ name: String(args.name ?? ''), path: String(args.path ?? '') });
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
      mockVaults.push({ name: String(args.name ?? ''), path: String(args.path ?? '') });
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
      // Nothing here is ever unreadable, so this is only reachable from a hand-crafted
      // call. Fail the way the real arm does rather than pretending it worked.
      throw new Error('not a currently-unreadable note');
    case 'commit':
      return false as T;
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

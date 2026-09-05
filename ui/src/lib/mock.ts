// Browser-only fallback: a tiny in-memory store that answers the same commands
// the Rust backend does, so the board is developable with `pnpm dev` and no
// desktop shell. It mirrors the backend's grouping (newest-first, first-seen
// column order) so what you see in the browser matches the real window. This
// file is deliberately NOT under src/renderers — status names live here, never
// in a renderer, which is the invariant the CI grep enforces.
import { parseStamp } from './stamp';
import { GIT_ASSETS_CEILING, humanSize } from './size';
import type {
  AssetStatus,
  BackupStatus,
  LatestBackup,
  BackupRun,
  Board,
  Column,
  NoteDetail,
  ObjectMeta,
  ImportCheck,
  ImportReport,
  PathCheck,
  PullResult,
  VaultInfo,
  ThemeInfo,
  ViewInfo,
  ViewResult,
  ConflictInfo,
  DuplicateFamily,
  Unrecorded,
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

/// A day in the **current** month, `offset` days from today, clamped to the month's bounds.
///
/// **The fixtures used to carry literal dates** (`due: '2026-07-11'`), and that made the whole
/// suite expire. The Agenda pane opens on the Calendar, which draws the *current* month — so once
/// the clock passed into August, a note due 2026-07-11 was simply not on screen, and
/// `App.flow.test.ts` went red for a reason that had nothing to do with the code. A test that
/// fails on a date gets deleted rather than fixed, which costs the coverage it was bought for.
///
/// Clamping to the month is the load-bearing part: "visible on this month's calendar" is what both
/// `pnpm dev` and the flow test actually depend on, and an unclamped `today − 3` falls off the
/// grid for the first three days of every month — a flake that would appear ten times a year and
/// be blamed on anything else.
function dayInThisMonth(offset: number): string {
  const now = new Date();
  const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
  const day = Math.min(Math.max(now.getDate() + offset, 1), lastDay);
  const month = String(now.getMonth() + 1).padStart(2, '0');
  return `${now.getFullYear()}-${month}-${String(day).padStart(2, '0')}`;
}

const notes: ObjectMeta[] = [
  makeNote({ preview: 'GAE lambda interacts badly with inner-loop adaptation', status: 'doing', tags: ['meta-rl'], props: { project: 'alpha' } }),
  makeNote({ preview: 'Draft the trust-region clipping ablation', status: 'todo', start: dayInThisMonth(-1), due: dayInThisMonth(4), hard: true, props: { project: 'alpha' } }),
  makeNote({ preview: 'Reply to reviewer 2', status: 'todo', due: dayInThisMonth(-3), hard: true, tags: ['neurips'], vault: 'lab' }),
  makeNote({ preview: 'Read the Muesli paper', status: 'todo', tags: ['reading'], props: { project: 'beta' } }),
  makeNote({ preview: 'Ship the second renderer', status: 'done', props: { project: 'beta' } }),
  makeNote({ preview: 'Weekly sync notes', start: `${dayInThisMonth(0)}T14:30`, due: `${dayInThisMonth(0)}T15:00`, tags: ['meeting'], vault: 'lab' }),
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

/// The same, over **bytes** — which is what content-addressing actually means.
///
/// `ingest` used to hash the *filename* and throw the bytes away, so no test could tell a photo
/// that arrived from one that arrived empty. That is not a hypothetical: every photo taken on the
/// phone was stored as zero bytes for days (`known-issues.md`), `ingest` hashed the empty string,
/// and every capture produced the same reference — a defect the entire suite was structurally
/// blind to. Hashing the bytes makes the arrival observable.
function fakeHashBytes(bytes: Uint8Array): string {
  let h = 0;
  for (let i = 0; i < bytes.length; i += 1) h = (Math.imul(31, h) + bytes[i]) | 0;
  // Length is folded in so a truncated payload cannot collide with the whole one.
  h = (Math.imul(31, h) + bytes.length) | 0;
  return (h >>> 0).toString(16).padStart(8, '0').repeat(8);
}

/// References whose bytes this mock has actually been given, so `asset_status` can answer
/// `has_blob` honestly for them. Everything else stays `false`, which is what the fixture notes'
/// `sha256:deadbeef` placeholders rely on.
const blobs = new Map<string, number>();

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

/// The one property/value the mock's saved **board** view filters out — i.e. the column a test
/// should find missing from `run_view('Active')` and present on the built-in board.
///
/// Exported so a test names it from here rather than hardcoding a status: what matters about that
/// column is that *a view removed it*, not which one it is. The board is group-by-anything and the
/// UI carries no status literals; a test that spelled one would be asserting the wrong thing.
export const MOCK_VIEW_HIDDEN = { key: 'status', value: 'done' } as const;

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
      // Comma only, matching `apply_property` — this file's own rule is that the mock must refuse
      // what the Rust refuses, or the UI suite passes against a backend that does not exist. When
      // tags became comma-separated so a tag could contain a space, this arm was left behind and
      // every UI test kept splitting on whitespace.
      n.tags = value ? value.split(',').map((t) => t.trim()).filter(Boolean) : [];
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
// Where each vault's media would go, and whether this machine can unlock any of it. Mutable so
// the backup panel's restic fields can be developed against the mock, exactly like `gitVaults`.
const mockRestic: Record<string, string | null> = {};
let mockResticPassword = false;
// When each vault was last snapshotted here. Empty at start, so the first thing `pnpm dev` shows
// is the "never backed up" state — the one that should worry somebody and therefore the one worth
// having on screen by default.
const mockLastBackup: Record<string, string> = {};

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
// `label` is what a vault is *called* when it has a remote (the repository name); `name` stays the
// identity. `lab` carries one and `personal` does not, so both paths are on screen under `pnpm dev` —
// the same reason `personal` starts with assets off and `lab` starts on.
let mockVaults: Array<{
  name: string;
  path: string;
  git_assets_max: number | null;
  supervision: { collect: boolean; publish: boolean };
  label: string | null;
  identity: { name: string; email: string } | null;
}> = [
  // **Both vaults carry an identity, and the default one must.** The welcome screen gates on
  // `vaults[0].identity`, and the rest of this mock describes a notebook already in use — notes,
  // boards, saved views, unpushed commits. A returning user has told git who they are, so leaving
  // this null would put a first-run screen in front of every test and every `pnpm dev` session,
  // which is a premise the mock does not otherwise hold. `Welcome.svelte`'s own tests supply the
  // null case; `create_vault` and `restore_vault` below still produce one, which is the honest
  // place for it.
  {
    name: 'personal',
    path: '/home/you/notes',
    git_assets_max: null,
    supervision: { collect: true, publish: false },
    label: null,
    identity: { name: 'Ada Lovelace', email: 'ada@example.org' },
  },
  {
    name: 'lab',
    path: '/home/you/lab-notes',
    git_assets_max: 2_000_000,
    supervision: { collect: true, publish: false },
    label: 'lab-notes',
    identity: { name: 'Ada Lovelace', email: 'ada@example.org' },
  },
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
  const n = Math.round(parseFloat(m[1]) * mult[m[2] ?? '']);
  // The backend refuses above the ceiling (`Descriptor::set_git_assets_max`), so the mock must
  // too — otherwise `pnpm dev` can reach a configuration the product forbids, which is the exact
  // drift this file's header promises not to have.
  if (n > GIT_ASSETS_CEILING) {
    throw new Error(
      `${humanSize(n)} is larger than ${humanSize(GIT_ASSETS_CEILING)} — without git-lfs an ` +
        `attachment that size is permanent history and most hosts refuse the push outright. ` +
        `Leave the heavy ones to the backup snapshot.`,
    );
  }
  return n;
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
let mockPaperSeq = 1;
/** Themes the mock vault holds. Empty by default: a vault that has never been themed is the
 *  ordinary case, and a component must render correctly with nothing here. */
let mockThemes: ThemeInfo[] = [];
const mockThemeCss = new Map<string, string>();
let mockSavedViews: ViewInfo[] = [
  { name: 'Recent notes', renderer: 'timeline', group_by: null },
  { name: 'Active', renderer: 'board', group_by: 'status' },
];
// **The assistant's capability, modelled separately from its switch.** The mock answered only
// `enabled`, so the panel's "can this machine run it at all" branch was never exercised — the same
// shape of gap that twice before let a defect through because the mock was too polite to fail.
// Flip this to see the not-installed row the way a released build shows it.
let mockAgentInstalled = true;
let mockTranscribeEnabled = false;
// The whisper runtime is a **second** capability, and the panel branches on it separately: the
// assistant can be perfectly installed while audio transcription still has nothing behind it.
// Flip this to see the row that names what is missing instead of offering a switch.
let mockTranscribeAvailable = true;
// **Not provisioned by default**, so `pnpm dev` opens on the state a new user actually meets: the
// assistant is available but its model has not been downloaded, which is the screen that asks
// before spending gigabytes. A mock that starts fully provisioned would hide the whole flow.
let mockProvisioned = false;
let mockProvisioning: {
  stage: 'runtime' | 'model' | 'projector' | 'ready' | 'failed';
  done: number;
  total: number | null;
  error: string | null;
} | null = null;

/// **A backend that can misbehave, because the real one does.**
///
/// The mock answered every command, always, immediately — so every rejection, slow-start and
/// no-answer-at-all path in the UI was untested by construction, and the one that mattered shipped:
/// a `list_vaults` that never answered rendered a blank screen on the owner's phone, and a
/// `list_vaults` that *refused* rendered the first-run "create your first vault" form at someone
/// with ten vaults. `known-issues.md` already argued the general case for one hand-written guard —
/// *"a mock that quietly accepts a write the backend would refuse is how the UI's rejection path
/// stays untested until a user finds it"* — this is that argument made into a mechanism.
///
/// Test-only: nothing in the app calls it, and with no faults registered `handle` behaves exactly
/// as it always did.
export type Fault = {
  /// The dispatch command name to interfere with, e.g. `list_vaults`.
  cmd: string;
  /// `reject` fails it, `hang` never settles (the phone's actual symptom), `delay` answers late.
  mode: 'reject' | 'hang' | 'delay';
  message?: string;
  ms?: number;
  /// Apply to this many calls, then let the command succeed. Omitted = forever.
  times?: number;
};
/// Make `record_unrecorded` answer the way a git refusal does: notes found, nothing committed, a reason.
/// The `Fault` machinery cannot express this — it is not an error, it is a *successful* reply that says
/// no, which is exactly why the UI mistook it for "nothing to do". Test-only.
let recordRefused = false;
export function setRecordRefused(on: boolean) {
  recordRefused = on;
}
/// Conflicts the mock cannot invent for itself: the marker-less kinds (delete/modify), which are the
/// ones the UI has to offer a *side* for rather than an editor. Test-only.
let mockConflicts: ConflictInfo[] = [];
export function setConflicts(list: ConflictInfo[]): void {
  mockConflicts = list.map((c) => ({ ...c }));
}
/// Notes the app forgot it wrote, per vault. Test-only.
/// Identical-note families. Test-only; the mock has no scan of its own.
let mockDuplicates: DuplicateFamily[] = [];
export function setDuplicates(list: DuplicateFamily[]): void {
  mockDuplicates = list.map((f) => ({ ...f }));
}
let mockUnopenedVaults: string[] = [];
/// Configured vaults that would not open, as `name: why`. Test-only.
export function setUnopenedVaults(list: string[]): void {
  mockUnopenedVaults = [...list];
}
let mockUnrecorded: Unrecorded[] = [];
export function setUnrecorded(list: Unrecorded[]): void {
  mockUnrecorded = list.map((u) => ({ ...u }));
}

/// Set (or clear) the **default** vault's committer — what the welcome screen gates on.
///
/// The fixtures describe a notebook already in use, so `personal` carries an identity and the
/// welcome screen stays out of the way of every other test. This is how a test says "nobody has
/// told git who they are yet" without a second mock of `list_vaults`, in the same shape as
/// [`faults`]. Call it from `beforeEach`: module state outlives a test.
export function setMockIdentity(identity: { name: string; email: string } | null): void {
  mockVaults[0].identity = identity;
  const git = gitVaults.find((v) => v.name === mockVaults[0].name);
  if (git) git.identity = identity;
}

/// Which OS `get_config` should claim to be running on.
///
/// Defaults to the desktop shape, because that is what almost every test wants and a suite that
/// silently ran as a phone would assert the wrong things everywhere. Flip it to `'ios'` to reach
/// the sideload notice, `'android'` for the other phone. Call it from `beforeEach`: module state
/// outlives a test, and [`reset`] puts it back.
let mockPlatform = 'linux';
export function setPlatform(os: string): void {
  mockPlatform = os;
}

let mockFaults: Fault[] = [];
export function faults(list: Fault[]): void {
  mockFaults = list.map((f) => ({ ...f }));
}
/// Call from `beforeEach`: module state outlives a test, because vitest isolates per *file*, not
/// per test — the trap `known-issues.md` records for `bodyOverrides` and the `seq` counter.
///
/// Deliberately does **not** touch the note list; use [`reset`] for that.
export function clearFaults(): void {
  mockFaults = [];
  mockConflicts = [];
  mockUnrecorded = [];
  mockUnopenedVaults = [];
  mockDuplicates = [];
}

/// The nine fixture notes exactly as they were built at import, so [`reset`] can put them back
/// **without re-running `makeNote`** — which would mint new ids and re-stamp `created`/`updated`
/// from a later `Date.now()`, shifting the dates other tests group by. Snapshot, not recipe.
const FIXTURES = notes.map((n) => JSON.parse(JSON.stringify(n)) as ObjectMeta);
const FIXTURE_SEQ = seq;
/// The vault list as imported. `create_vault`/`forget_vault` mutate `mockVaults` in place, so a
/// test that creates one leaks it into every later test in the file unless `reset()` puts it back.
const FIXTURE_VAULTS = JSON.parse(JSON.stringify(mockVaults)) as typeof mockVaults;

/// Put the mock back to its just-imported state.
///
/// **The gap this closes.** There was no way to undo a seeded note. `clearFaults()` deliberately
/// leaves `notes` and `bodyOverrides` alone, and neither was exported, so a test that added 500
/// notes leaked them into every later test *in the same file* — which is why no test had ever
/// tried. A suite that cannot seed a realistic vault cannot test the app at the size the owner
/// actually runs it.
/// **Every mutable cell in this module, or the name is a lie.** The first version restored the
/// notes and forgot `recordRefused`, `mockVaults`, `mockAgentEnabled` and `mockTranscribeEnabled` —
/// so a test that turned the agent on, or refused a recording, silently changed the starting
/// conditions of every later test in the same file. That is the precise trap the comment on
/// `clearFaults` warns about, reintroduced by the function written to fix it.
export function reset(): void {
  clearFaults();
  notes.length = 0;
  notes.push(...FIXTURES.map((n) => JSON.parse(JSON.stringify(n)) as ObjectMeta));
  bodyOverrides.clear();
  blobs.clear();
  acceptedProposals.clear();
  seq = FIXTURE_SEQ;
  recordRefused = false;
  mockAgentEnabled = false;
  mockTranscribeEnabled = false;
  mockVaults = JSON.parse(JSON.stringify(FIXTURE_VAULTS)) as typeof mockVaults;
  mockPlatform = 'linux';
}

/// Fill the vault to a realistic size.
///
/// The phone's problems are size problems — a full-corpus scan is free at nine notes and is
/// seconds at a few thousand — so a suite that only ever sees nine notes cannot see them at all.
/// Notes are pushed directly rather than through `handle('capture')`: seeding is setup, and it
/// should not be the thing under test (the same reasoning as `fm-core/tests/perf.rs`, which writes
/// its 10k notes straight to disk so only the query is timed).
///
/// Returns the ids in creation order, so a test can open a known one.
export function seed(opts: { notes?: number; bodyBytes?: number; vault?: string } = {}): string[] {
  const count = opts.notes ?? 0;
  const vault = opts.vault ?? 'personal';
  const ids: string[] = [];
  for (let i = 0; i < count; i += 1) {
    // Varied on purpose: every note identical would let a grouping or sort regression pass.
    const n = makeNote({
      preview: `seeded note ${i} about gradients and estimators`,
      title: `Seeded ${i}`,
      status: ['todo', 'doing', 'done'][i % 3],
      tags: i % 4 === 0 ? ['seeded'] : [],
      vault,
    });
    notes.push(n);
    ids.push(n.id);
  }
  if (opts.bodyBytes && ids.length) {
    // One genuinely large note, on the first seeded id. Not `'x'.repeat(n)`: a body of identical
    // bytes lets a renderer or a diff regress invisibly, and the editor path we care about is
    // "many lines", not "one enormous line".
    const line = 'The advantage estimate leaks across the meta-update boundary. ';
    const lines: string[] = [];
    let size = 0;
    for (let i = 0; size < opts.bodyBytes; i += 1) {
      const l = `${i}. ${line}`;
      lines.push(l);
      size += l.length + 1;
    }
    bodyOverrides.set(ids[0], lines.join('\n'));
  }
  return ids;
}

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const fault = mockFaults.find((f) => f.cmd === cmd);
  if (fault) {
    if (fault.times !== undefined) {
      if (fault.times <= 1) mockFaults = mockFaults.filter((f) => f !== fault);
      else fault.times -= 1;
    }
    if (fault.mode === 'hang') return new Promise<T>(() => {});
    if (fault.mode === 'reject') throw new Error(fault.message ?? `${cmd}: refused`);
    await new Promise((r) => setTimeout(r, fault.ms ?? 5000));
  }
  switch (cmd) {
    case 'agent_status':
      return {
        enabled: mockAgentEnabled,
        transcribe: mockTranscribeEnabled,
        installed: mockAgentInstalled,
        why: mockAgentInstalled
          ? ''
          : 'The study assistant runs on Linux today. Everything else in formicaria works normally here — your notes, search, boards and backup are unaffected.',
        transcribe_available: mockTranscribeAvailable,
        transcribe_fetchable: !mockTranscribeAvailable,
        provisioned: mockProvisioned,
        provisioned_bytes: mockProvisioned ? 2_497_281_664 : 0,
        provisioning: mockProvisioning,
      } as T;
    case 'remove_agent_model': {
      // The real thing turns the assistant off first, then deletes; the mock does the same so the
      // panel's state after removal is developable.
      const freed = mockProvisioned ? 2_497_281_664 : 0;
      mockAgentEnabled = false;
      mockProvisioned = false;
      mockProvisioning = null;
      return { freed } as T;
    }
    case 'agent_models':
      // The real catalogue's shape, sizes included — the numbers are what `agents/models.toml`
      // records, so the dev build shows the same figures a user would be asked to accept.
      return [
        {
          name: 'qwen3-vl-4b',
          bytes: 2_497_281_664,
          license: 'Apache-2.0',
          vision: true,
          mmproj_bytes: 836_180_256,
          default: true,
        },
        {
          name: 'lfm2.5-1.2b',
          bytes: 1_400_000_000,
          license: 'LFM Open License v1.0',
          vision: false,
          mmproj_bytes: null,
          default: false,
        },
      ] as T;
    case 'set_agent':
      // Refuses exactly as the server does, so a test can see the refusal rather than a cheerful ok.
      if (args.enabled && !mockAgentInstalled) {
        throw new Error('The study assistant is not installed on this machine.');
      }
      mockAgentEnabled = Boolean(args.enabled);
      // Model the first enable: turning it on when nothing is provisioned starts a download rather
      // than simply flipping to on. Two ticks of `agent_status` later it is ready — enough for the
      // panel's polling to be developed against something that moves.
      if (mockAgentEnabled && !mockProvisioned) {
        mockProvisioning = { stage: 'model', done: 0, total: 2_497_281_664, error: null };
        setTimeout(() => {
          mockProvisioning = { stage: 'model', done: 1_200_000_000, total: 2_497_281_664, error: null };
        }, 400);
        setTimeout(() => {
          mockProvisioned = true;
          mockProvisioning = { stage: 'ready', done: 0, total: null, error: null };
        }, 1200);
      }
      if (!mockAgentEnabled) mockProvisioning = null;
      return { ok: true } as T;
    case 'set_transcribe':
      // Refuses like the server: without the runtime staged, turning this on transcribes nothing.
      if (args.transcribe && !mockTranscribeAvailable) {
        throw new Error('The speech-to-text runtime is not on this machine.');
      }
      mockTranscribeEnabled = Boolean(args.transcribe);
      return { ok: true } as T;
    case 'agent_activity_poll':
      return { active: false } as T;
    case 'agents':
      // In the mock, the "enabled" toggle stands in for a live agent, so the @-picker is demoable.
      return { agents: mockAgentEnabled ? ['qwen3-vl-4b'] : [] } as T;
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
      // A note is in conflict iff its body carries both markers — same rule as the server. The mock
      // has no git, so every conflict it can invent is a marker one; `mockConflicts` is how a test
      // stands in for the kinds that have **no** markers (a delete/modify), which is the case the UI
      // got wrong for a week and which no marker-based mock can ever produce.
      const marked = (s: string) => /^<{7}/m.test(s) && /^>{7}/m.test(s);
      const scanned: ConflictInfo[] = notes
        .filter(isNote)
        .filter((n) => marked(bodyOverrides.get(n.id) ?? n.preview))
        .sort((a, b) => b.updated.localeCompare(a.updated))
        .map((note) => ({
          note,
          path: `notes/${note.id}.md`,
          vault: note.vault,
          code: 'UU',
          what: 'Both sides edited this note; both versions are marked in the text.',
          has_markers: true,
        }));
      return [...mockConflicts, ...scanned] as T;
    }
    case 'resolve_conflict': {
      // The server refuses "edited" while markers remain (`merge::has_conflict_markers`); the mock
      // mirrors that refusal, because a mock that accepts what the backend rejects is how the UI's
      // rejection path stays untested until a user finds it.
      if (args.keep === 'edited') {
        const body = String(bodyOverrides.get(String(args.id ?? '')) ?? '');
        if (/^<{7}/m.test(body) && /^>{7}/m.test(body)) {
          throw new Error(`${args.path}: still has conflict markers`);
        }
      }
      // Resolving drops it off the list, exactly as the server's does (the path stops being
      // unmerged), so a test can assert the surface empties rather than only that a call happened.
      mockConflicts = mockConflicts.filter((c) => c.path !== String(args.path));
      return { resolved: String(args.path) } as T;
    }
    case 'forget_vault': {
      // Mirrors the server: unregister, never delete. `notes` is what is being left behind.
      const name = String(args.name);
      if (!name) throw new Error('which vault? forget_vault needs a name');
      const before = mockVaults.length;
      const left = notes.filter((n) => n.vault === name).length;
      mockVaults = mockVaults.filter((v) => v.name !== name);
      if (mockVaults.length === before) throw new Error(`no vault named '${name}'`);
      return { forgotten: name, path: `/vaults/${name}`, notes: left, remote: null, vaults: mockVaults } as T;
    }
    case 'duplicates':
      return mockDuplicates as T;
    case 'prune_duplicates': {
      // Mirrors the server's refusal: never delete what git does not have.
      if (mockUnrecorded.some((u) => u.vault === String(args.vault) && u.count > 0)) {
        throw new Error(
          'these copies are not in git history yet, and deleting one would be unrecoverable. Record them first',
        );
      }
      const mine = mockDuplicates.filter((f) => f.vault === String(args.vault));
      const removed = mine.reduce((n, f) => n + f.extras.length, 0);
      mockDuplicates = mockDuplicates.filter((f) => f.vault !== String(args.vault));
      return { removed, kept: mine.length } as T;
    }
    case 'unrecorded':
      return mockUnrecorded as T;
    case 'record_unrecorded': {
      const hit = mockUnrecorded.find((u) => u.vault === String(args.vault));
      // **The refusal is reachable here too.** The server can find notes and still be declined by git
      // (a note mid-merge, no identity); reporting that as an empty vault is the bug that hid 147
      // unrecorded notes behind a reassuring message. `Fault.recordRefused` lets a test drive it.
      if (hit && recordRefused) {
        return {
          committed: false,
          notes: hit.count,
          reason: '1 note(s) in this vault are mid-merge. Git refuses to commit anything until those are resolved — open Conflicts and settle them first.',
        } as T;
      }
      mockUnrecorded = mockUnrecorded.filter((u) => u.vault !== String(args.vault));
      return { committed: !!hit, notes: hit?.count ?? 0, reason: '' } as T;
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
      // A GONE proposal note (rejected elsewhere, or a stale list) → nothing to show, not an error —
      // the same tolerance the real backend now has (what a client sees before its list refreshes). A
      // note that exists but is not a proposal is still an error, exactly as on the backend.
      if (!prop) return { exists: false, declined: false, files: [], patch: '' } as T;
      if (!isProposal(prop)) throw new Error('not a proposal');
      const declined = prop.props?.declined === 'true';
      // Declined (branch dropped, note kept) or accepted (merged) → the branch is gone; the note
      // outlives it, so the diff reports "nothing to show", distinguishing declined from merged.
      if (declined || acceptedProposals.has(prop.id)) {
        return { exists: false, declined, files: [], patch: '' } as T;
      }
      // The mock has no git, so it returns a stand-in patch (there is no real branch to diff). The
      // real backend diffs `proposal/<id>` against `main`. Shape matches the server's `ProposalDiff`.
      return {
        exists: true,
        declined: false,
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
      // **A declined proposal is refused here too** (2026-09-04). `proposal_content` and
      // `proposal_for` in this file already check `declined`; this arm did not, matching a real
      // gap in `commands::accept_proposal` that let a peer's rejected text merge into `main`. Both
      // were fixed together on purpose: a mock that accepts what the backend refuses lets a UI test
      // go green for the wrong reason, which is the drift `mock.ts`'s own header warns about.
      if (prop.props?.declined === 'true') return { outcome: 'already_gone' } as T;
      acceptedProposals.add(prop.id);
      return { outcome: 'merged' } as T;
    }
    case 'proposal_shown': {
      const pr = notes.find((n) => n.id === String(args.id) && isProposal(n));
      if (!pr) throw new Error('not a proposal');
      // First sighting only — re-stamping would erase how long the reviewer actually took.
      if (!pr.props?.shown) {
        pr.props = { ...pr.props, shown: new Date().toISOString() };
      }
      return undefined as T;
    }
    case 'reject_proposal': {
      // Reject drops the branch but KEEPS the proposal note as a declined record (a proposal is
      // immortal, like a merged one). The mock marks it `declined`; the real backend also deletes the
      // `proposal/<id>` branch. main is untouched either way.
      const p = notes.find((n) => n.id === String(args.id) && isProposal(n));
      if (!p) throw new Error('not a currently-open proposal');
      p.props = { ...p.props, declined: 'true' };
      const rw = String(args.why ?? '').trim();
      if (rw) p.props = { ...p.props, declined_why: rw };
      return undefined as T;
    }
    case 'create_proposal': {
      const target = notes.find((n) => n.id === String(args.id));
      if (!target) throw new Error('no such note');
      // Living-PR (mirrors the real upsert): refine the note's existing OPEN proposal instead of
      // stacking a new one. The mock has no git, so it records the proposal *note*; the real backend
      // also builds/revises the `proposal/<id>` branch and enforces the vault's size guardrails.
      const existing = notes.find(
        (n) =>
          isProposal(n) &&
          n.props?.targets === target.id &&
          !acceptedProposals.has(n.id) &&
          n.props?.declined !== 'true',
      );
      if (existing) {
        // Refine: update the stored proposed body (the real backend revises the branch).
        const w = String(args.why ?? '').trim();
        const kd = String(args.kind ?? '').trim();
        existing.props = {
          ...existing.props,
          proposedBody: String(args.body ?? ''),
          ...(w ? { why: w } : {}),
          ...(kd ? { kind: kd } : {}),
        };
        return existing as T;
      }
      const title = target.title ?? 'note';
      const p = makeNote({ preview: `Proposed change to ${title}`, title: `Proposal: ${title}`, vault: target.vault });
      p.props = { proposes: `branch:proposal/${p.id}`, targets: target.id, proposedBody: String(args.body ?? '') };
      notes.unshift(p);
      return p as T;
    }
    case 'proposal_content': {
      // The proposed note (host + title + body) — the mock stores the proposed body on the note; the
      // real backend reads it off the `proposal/<id>` branch.
      const prop = notes.find((n) => n.id === String(args.id) && isProposal(n));
      if (!prop || acceptedProposals.has(prop.id) || prop.props?.declined === 'true') return null as T;
      const host = prop.props?.targets ?? '';
      const target = notes.find((n) => n.id === host);
      return { host, title: target?.title ?? 'note', body: prop.props?.proposedBody ?? '' } as T;
    }
    case 'proposal_for': {
      // The note's current open proposal (its PR), or null.
      const host = String(args.id);
      const p = notes.find(
        (n) =>
          isProposal(n) &&
          n.props?.targets === host &&
          !acceptedProposals.has(n.id) &&
          n.props?.declined !== 'true',
      );
      return (p ? p.id : null) as T;
    }
    // Every root any message points at, with its count — one pass, mirroring `thread_roots`. The
    // self-message of a first-class discussion is not a reply, so it registers the root at zero.
    case 'thread_roots': {
      const counts = new Map<string, number>();
      for (const n of notes) {
        const ref = typeof n.props?.thread_of === 'string' ? n.props.thread_of : '';
        const root = ref.startsWith('note:') ? ref.slice('note:'.length) : '';
        if (!root) continue;
        counts.set(root, (counts.get(root) ?? 0) + (n.id === root ? 0 : 1));
      }
      return [...counts].map(([id, count]) => ({ id, count })) as T;
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
      // **`excerpt` is filled here and nowhere else**, mirroring `commands::recent` — the feed is
      // the only surface that reads past the first line. The mock has no bodies, so it echoes the
      // preview; what the mirror has to preserve is *which command carries the field*, because a
      // renderer that finds it on a board card would be reading a payload the server never sends.
      const recent: ObjectMeta[] = notes
        .filter(isNote)
        .sort((a, b) => b.created.localeCompare(a.created))
        .map((n) => ({ ...n, excerpt: n.preview }));
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
      // insert a reference.
      const name = String(args.name ?? 'asset');
      // The asset joins the audience of the note it was dropped on — same rule the real
      // backend follows, mirrored so `pnpm dev` can't show a flow the backend refuses.
      const vault = mockVault(args.vault).name;
      // **Address the bytes when there are bytes.** The phone hands them over (base64 through
      // `fm_ingest`, decoded by the harness); the browser dev path has none and keeps the old
      // name-derived hash so `pnpm dev` still "dedups" a re-added file. The difference is the
      // point: with a byte hash, a photo that arrives truncated or empty gets a *different*
      // reference and a test can see it.
      const bytes = args.bytes as Uint8Array | undefined;
      const hash = bytes ? fakeHashBytes(bytes) : fakeHash(name);
      if (bytes) blobs.set(`sha256:${hash}`, bytes.length);
      const n = makeNote({
        preview: name,
        type: 'asset',
        title: name,
        assets: [`sha256:${hash}`],
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
      // Honest for anything this mock was actually given bytes for; `false` for the fixture
      // notes' placeholder references, which is what makes the "no bytes in vault" path
      // reachable in `pnpm dev` and in the render tests.
      const ref = String(args.reference ?? '');
      const has = blobs.has(ref);
      const status: AssetStatus = {
        has_blob: has,
        has_thumb: false,
        mime: has ? 'image/jpeg' : null,
      };
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
    case 'set_supervision': {
      // Two independent answers: an omitted one keeps its current value, so a UI can offer them as
      // separate switches without either implying the other.
      const v = mockVaults.find((x) => x.name === String(args.vault)) ?? mockVaults[0];
      v.supervision = {
        collect: typeof args.collect === 'boolean' ? args.collect : v.supervision.collect,
        publish: typeof args.publish === 'boolean' ? args.publish : v.supervision.publish,
      };
      return mockVaults.map((x, i) => ({ ...x, default: i === 0 })) as T;
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
    case 'check_import': {
      // Same rule as `check_path` above: mirrors the real policy closely enough to develop the
      // form against, and no further. The server owns `ok`; this must never become a second
      // opinion. A path containing "logseq" or "obsidian" stands in for detection.
      const source = String(args.source ?? '');
      const vault = String(args.vault ?? '');
      const format = /logseq/i.test(source)
        ? 'logseq'
        : /obsidian/i.test(source)
          ? 'obsidian'
          : null;
      const destination_ok = vault ? mockVaults.some((v) => v.name === vault) : true;
      const problem = !source.trim()
        ? 'there is no folder at that path'
        : !format
          ? 'that folder does not look like a Logseq graph or an Obsidian vault'
          : !destination_ok
            ? `no vault named '${vault}'`
            : null;
      const check: ImportCheck = {
        format,
        label: format === 'logseq' ? 'Logseq' : format === 'obsidian' ? 'Obsidian' : null,
        pages: format ? 12 : 0,
        journals: format === 'logseq' ? 4 : 0,
        attachments: format ? 3 : 0,
        attachmentBytes: format ? 2_400_000 : 0,
        leftBehind: format === 'obsidian' ? [{ kind: 'canvas', count: 1 }] : [],
        problem,
        ok: problem === null,
      };
      return check satisfies ImportCheck as T;
    }
    case 'run_import': {
      const report: ImportReport = {
        format: /obsidian/i.test(String(args.source ?? '')) ? 'obsidian' : 'logseq',
        notes: 16,
        stubs: args.stubs ? 2 : 0,
        alreadyImported: 0,
        attachments: 3,
        deduped: 0,
        links: 21,
        dangling: 2,
        danglingNames: ['Someday', 'Reading List'],
        blocks: 4,
        blocksUnresolved: 0,
        renamedProperties: 1,
        leftBehind: [],
        warnings: [],
        recorded: true,
        vault: String(args.vault || args.name || 'notes'),
      };
      return report satisfies ImportReport as T;
    }
    case 'config':
      // Shaped like the real thing, including the awkward parts — a null vault_list and an
      // unwritable one are exactly the states the panel must render honestly, and a mock that
      // only ever returns the happy case is how those go untested.
      return {
        // `dev` is what a local build genuinely reports (`option_env!("FM_VERSION")` unset), so
        // the mock says the same rather than inventing a release number that never existed.
        version: 'dev',
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
        // **False on purpose.** The dev loop should show the not-available wording by default,
        // because that is the state a released machine is most often in and the one that used to
        // be invisible. A happy-path mock is how the silence lasted this long.
        pdf_text: false,
        // **Derived from the platform, because the backend cannot produce any other combination.**
        // `vault_root()` is `Some` exactly where the shell sets `FM_VAULT_ROOT`, i.e. on a phone —
        // so a mock that let a test be `platform: 'android'` with `vault_root: null` would let it
        // assert against a state that cannot exist. It did, once, and the test that caught nothing
        // looked like a failing feature.
        vault_root: mockPlatform === 'ios' || mockPlatform === 'android'
          ? '/data/app/dev.formicaria.notes/vaults'
          : null,
        platform: mockPlatform,
        ca_bundle: null, // the desktop shape: the system store is used, none is built
      } as T;
    case 'create_vault': {
      mockVaults.push({
        name: String(args.name ?? ''),
        path: String(args.path ?? ''),
        // A new or acquired vault has no opinion yet, which means notes only — the real default.
        git_assets_max: null,
        // Collect locally, publish nothing: publication cannot be recalled, so it is never assumed.
        supervision: { collect: true, publish: false },
        // A freshly created vault has no remote, so nothing to label it with — it keeps its name.
        label: null,
        // Nor a committer: git has not been told who you are, which is what the welcome screen asks.
        identity: null,
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
        // Collect locally, publish nothing: publication cannot be recalled, so it is never assumed.
        supervision: { collect: true, publish: false },
        // A clone *does* have a remote, so it gets the repository's name as its label.
        label: String(args.url ?? '')
          .trim()
          .replace(/\/$/, '')
          .split(/[/:]/)
          .pop()
          ?.replace(/\.git$/, '') || null,
        // A clone names its committer as part of the form, which is why this one is never null.
        identity: {
          name: String(args.gitName ?? ''),
          email: String(args.gitEmail ?? ''),
        },
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
        // Collect locally, publish nothing: publication cannot be recalled, so it is never assumed.
        supervision: { collect: true, publish: false },
        // A restored vault has no remote until one is set.
        label: null,
        // The restore brings back a repository whose commits already carry a committer, but this
        // machine's git has not been told who is writing *now* — so the welcome screen still asks.
        identity: null,
      });
      const restored: VaultInfo[] = mockVaults.map((v, i) => ({ ...v, default: i === 0 }));
      return restored as T;
    }
    // Saved views. A real `.view` lives in the vault and is parsed server-side, which the mock
    // does not model — but it ships two, and the *second* one matters: a **filtered board**.
    //
    // Until it existed, neither the dev build nor any test could show the shape that cost the owner
    // an afternoon (2026-08-24): a `view: board` draws through the same renderer as the Board pane,
    // so a filter that removes a whole column removes it with nothing on screen to say so. A mock
    // that only ever answered an unfiltered timeline could not reproduce that, which is precisely
    // why it went unnoticed.
    // The paste path, mirroring the Rust closely enough for the dialog's tests to mean something:
    // a BibTeX title wins, then an identifier, then the raw text as a title.
    case 'create_paper': {
      const input = String(args.input ?? '').trim();
      const bib = /title\s*=\s*[{"]([^}"]+)[}"]/i.exec(input);
      const doi = /\b(10\.\d{4,9}\/\S+)/.exec(input);
      const arx = /arxiv\.org\/(?:abs|pdf)\/([\w.\/]+?)(?:\.pdf)?$|arxiv:\s*([\w.\/]+)/i.exec(input);
      const props: Record<string, unknown> = {};
      if (doi) props.doi = doi[1];
      if (arx) props.arxiv = arx[1] ?? arx[2];
      const title = bib ? bib[1] : doi || arx || !input ? null : input;
      const meta: ObjectMeta = {
        id: `M0CKPAPER${String(mockPaperSeq++).padStart(16, '0')}`,
        type: 'note',
        title,
        preview: title ?? '',
        status: null,
        due: null,
        start: null,
        hard: false,
        created: new Date().toISOString(),
        updated: new Date().toISOString(),
        tags: ['paper'],
        assets: [],
        props,
        vault: String(args.vault || 'personal'),
      };
      return meta as T;
    }
    case 'paper_bibtex': {
      return '@article{mock2026,\n  title = {A mock paper}\n}\n' as T;
    }
    // Writable in the mock too, so the dev loop and tests exercise the path that used to require
    // a text editor. Kept in a module-level list so save/delete actually change what list_views
    // returns — a mock that accepted a write and reported the old list would hide the bug.
    case 'save_view': {
      const name = String(args.name ?? '').trim();
      if (!name) throw new Error('a view needs a name');
      mockSavedViews = mockSavedViews.filter((v) => v.name !== name);
      mockSavedViews.push({
        name,
        renderer: String(args.view ?? 'board') as ViewInfo['renderer'],
        group_by: args.group_by ? String(args.group_by) : null,
      });
      return mockSavedViews.slice() as T;
    }
    // Themes. The mock keeps them in memory the same way it keeps views, so a component test can
    // save one, list it and read it back without a server — which is the whole reason the theme
    // surface is four commands rather than a static URL.
    case 'list_themes':
      return mockThemes.slice() as T;
    case 'read_theme': {
      const name = String(args.name ?? '');
      const css = mockThemeCss.get(name);
      if (css === undefined) throw new Error(`there is no theme called '${name}'`);
      return css as T;
    }
    case 'save_theme': {
      const name = String(args.name ?? '')
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9 _-]/g, '-')
        .split(/\s+/)
        .join('-');
      const css = String(args.css ?? '');
      if (!name) throw new Error('a theme needs a name');
      // The same ceiling the backend enforces, so a test can exercise the refusal.
      if (css.length > 128 * 1024) throw new Error('this theme is too big, and the limit is 128 KB');
      mockThemes = mockThemes.filter((t) => t.name !== name);
      mockThemes.push({ name, vault: 'personal', bytes: css.length });
      mockThemeCss.set(name, css);
      return mockThemes.slice() as T;
    }
    case 'rename_theme': {
      const from = String(args.from ?? '');
      const to = String(args.to ?? '');
      const t = mockThemes.find((x) => x.name === from);
      if (!t) throw new Error(`there is no theme called '${from}'`);
      if (mockThemes.some((x) => x.name === to)) throw new Error('there is already a theme called ' + to);
      mockThemeCss.set(to, mockThemeCss.get(from) ?? '');
      mockThemeCss.delete(from);
      mockThemes = mockThemes.map((x) => (x.name === from ? { ...x, name: to } : x));
      return mockThemes.slice() as T;
    }
    case 'delete_theme': {
      const name = String(args.name ?? '');
      mockThemes = mockThemes.filter((t) => t.name !== name);
      mockThemeCss.delete(name);
      return mockThemes.slice() as T;
    }
    case 'rename_view': {
      const from = String(args.from ?? '');
      const to = String(args.to ?? '');
      if (mockSavedViews.some((v) => v.name === to)) throw new Error('there is already a view called ' + to);
      mockSavedViews = mockSavedViews.map((v) => (v.name === from ? { ...v, name: to } : v));
      return mockSavedViews.slice() as T;
    }
    case 'delete_view': {
      mockSavedViews = mockSavedViews.filter((v) => v.name !== String(args.name ?? ''));
      return mockSavedViews.slice() as T;
    }
    case 'list_views': {
      const views: ViewInfo[] = mockSavedViews.slice();
      return views as T;
    }
    case 'run_view': {
      // The board view answers a board with its filtered column genuinely absent — the same thing
      // the server does, and the only way a test can assert that the column comes back.
      if (String(args.name ?? '') === 'Active') {
        const full = buildBoard(MOCK_VIEW_HIDDEN.key);
        const result: ViewResult = {
          name: 'Active',
          renderer: 'board',
          group_by: MOCK_VIEW_HIDDEN.key,
          // The words ride with the payload, exactly as the Rust sends them.
          filters: [`${MOCK_VIEW_HIDDEN.key} is not ${MOCK_VIEW_HIDDEN.value}`],
          board: {
            group_by: MOCK_VIEW_HIDDEN.key,
            columns: full.columns.filter((c) => c.value !== MOCK_VIEW_HIDDEN.value),
          },
        };
        return result as T;
      }
      // Reuse the mock's own note set; the sample view just lists them like the timeline.
      const rows = notes.filter(isNote).sort((a, b) => b.created.localeCompare(a.created));
      const result: ViewResult = {
        name: String(args.name ?? ''),
        renderer: 'timeline',
        group_by: null,
        filters: [],
        rows,
      };
      return result as T;
    }
    case 'ping':
      // Nothing writes this vault but us, so it never moves under the app. `git: true`
      // because the mock models a working machine; the no-git path is exercised for real.
      // `generation: 0` pairs with `changed: false`: the mock vault never moves, so a client
      // beating against it keeps sending `since: 0` and keeps being told it is current.
      return {
        changed: false,
        generation: 0,
        git: true,
        restic: true,
        skipped: [],
        // Every configured vault opens in the mock; `setUnopenedVaults` is how a test stands in for
        // one that does not, the same way `setConflicts` stands in for a conflict git alone can make.
        unopened_vaults: mockUnopenedVaults,
      } as T;
    case 'open_skipped':
    case 'read_skipped':
    case 'resolve_skipped':
      // Nothing here is ever unreadable, so these are only reachable from a hand-crafted
      // call. Fail the way the real arms do rather than pretending they worked.
      throw new Error('not a currently-unreadable note');
    case 'commit': {
      // **A commit records what git did not have**, including notes an earlier process wrote and
      // never staged (`App::load` re-adopts them). The mock used to answer a flat "nothing to
      // commit", which meant no test could see that the "not in history" count is a fact about git
      // that a commit changes. The mock vault is never mid-merge, so there are no conflicts.
      const vault = String(args.vault);
      const had = mockUnrecorded.find((u) => u.vault === vault);
      mockUnrecorded = mockUnrecorded.filter((u) => u.vault !== vault);
      return { committed: !!had, conflicts: [] } as T;
    }
    case 'backup': {
      // A snapshot the mock actually remembers, so `backup_latest` below has something true to
      // say afterwards. Without this the panel's "last backed up" line could only ever be
      // developed against "never", which is the one state that needs no design.
      const v = mockVault(args.vault);
      if (mockRestic[v.name] && mockResticPassword) {
        mockLastBackup[v.name] = new Date().toISOString();
      }
      // **What went in.** The command answered `void` until 2026-09-05, so the panel had nothing
      // to say and said a fixed phrase instead. `contents: null` — the snapshot was written and
      // restic did not describe it — is a state the backend can produce and this mock therefore
      // must be able to, but it is not produced at random: a branch that appears on some runs and
      // not others is one nobody develops against. Set it by hand to see that line.
      const run: BackupRun = {
        vault: v.name,
        notes_dir: 'notes',
        blobs: true,
        contents: {
          id: 'a1b2c3d4',
          files_new: 3,
          files_changed: 1,
          files_unmodified: 214,
          bytes_processed: 48_200_000,
          bytes_added: 1_900_000,
        },
      };
      return run as T;
    }
    // **Three answers, not two.** `unavailable` is "this machine cannot tell you"; `id: null`
    // with no `unavailable` is "the repository opened and has never been written to". Collapsing
    // them is exactly the mistake the real command is shaped to prevent, so the mock keeps them
    // apart too — a mock that permits a state the backend cannot produce costs more than none.
    case 'backup_latest': {
      const v = mockVault(args.vault);
      const repo = mockRestic[v.name] ?? null;
      const unavailable = !repo
        ? 'this vault has no media-backup repository configured, so there is nothing to ask'
        : !mockResticPassword
          ? 'no restic password is set on this machine, so the repository cannot be opened'
          : null;
      const time = unavailable ? null : (mockLastBackup[v.name] ?? null);
      const latest: LatestBackup = {
        vault: v.name,
        id: time ? 'a1b2c3d4' : null,
        time,
        paths: time ? [`/home/you/${v.name}/notes`, `/home/you/${v.name}/blobs`] : [],
        unavailable,
      };
      return latest as T;
    }
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
          restic_repo: mockRestic[v.name] ?? null,
          // The real rule: restic installed, this vault has a repo, and a password is set. All
          // three, because "ready" must mean the backup would actually run.
          restic_ready: !!mockRestic[v.name] && mockResticPassword,
          // Read from `mockVaults`, never a second literal: Settings writes the limit there and
          // the backup panel reads it here, so the two surfaces must not be able to disagree
          // under `pnpm dev` — which is the whole defect this field exists to fix. `personal`
          // starts off and `lab` starts on, so both sentences are on screen.
          git_assets_max: mockVaults.find((m) => m.name === v.name)?.git_assets_max ?? null,
        })),
        git: true,
        restic: true,
        restic_password_set: mockResticPassword,
      };
      return status as T;
    }
    // Media backup, modelled with the two things that actually go wrong: restic missing, and a
    // password that was never set. A mock that always succeeded would leave the panel's honest
    // branches unexercised — the failure this file has been caught by twice.
    case 'set_restic_repo': {
      const v = mockVault(args.vault);
      const repo = String(args.repo ?? '').trim();
      mockRestic[v.name] = repo || null;
      return handle<T>('backup_status', {});
    }
    case 'set_restic_password': {
      if (!String(args.password ?? '').trim()) {
        throw new Error('an empty password would lock you out of your own backups');
      }
      mockResticPassword = true;
      return handle<T>('backup_status', {});
    }
    case 'clear_restic_password':
      mockResticPassword = false;
      return handle<T>('backup_status', {});
    // The welcome screen's first call. Modelled because the screen's whole promise is that a
    // failing *remote* leaves the name standing — which is only visible if the two are separate
    // here as well.
    case 'set_identity': {
      const v = mockVault(args.vault);
      const name = String(args.name ?? '').trim();
      const email = String(args.email ?? '').trim();
      if (!name || !email) throw new Error('a committer needs both a name and an email');
      v.identity = { name, email };
      const vaults = mockVaults.find((m) => m.name === v.name);
      if (vaults) vaults.identity = { name, email };
      return undefined as T;
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

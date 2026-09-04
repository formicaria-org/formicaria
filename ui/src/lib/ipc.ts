import type {
  Config,
  AssetStatus,
  BackupStatus,
  Board,
  CommitResult,
  EditEvent,
  GitAuth,
  ImportCheck,
  ImportReport,
  DiscussionSummary,
  NoteDetail,
  ObjectMeta,
  PathCheck,
  ProposalDiff,
  ProposalContent,
  PullResult,
  ThreadView,
  RemoteProbe,
  VaultInfo,
  ThemeInfo,
  ViewInfo,
  ViewResult,
  ConflictInfo,
  DuplicateFamily,
  Unrecorded,
  LatestBackup,
} from './types';
import * as mock from './mock';
import { blobBase } from './blobBase';
import { isPhone } from './platform';
import type { ShareStatus } from './remote';

// Two backends, one contract:
//   • Production browser (served by `fm-serve`) → the Rust commands over HTTP
//     (POST /api/<cmd>) against the real vault.
//   • `pnpm dev` / tests → an in-memory mock, so the UI stays developable with no
//     backend at all.
// `import.meta.env.PROD` is true only in the built bundle, so dev and tests hit
// the mock while the shipped bundle hits real data over HTTP.
// Is this bundle running inside the Tauri shell rather than a browser? Tauri v2 puts
// `__TAURI_INTERNALS__` on `window` before any of our code runs; `isPhone()` reads it.
//
// **The order matters, and it is the trap.** A Tauri build is `import.meta.env.PROD`, so
// without this branch *first* the app would take the HTTP path and `fetch('/api/…')` against a
// server that does not exist on the device. Same bundle, three backends, and the most specific
// one has to win.
//
// Read through `platform.ts` at call time rather than captured in a `const` here — see that file
// for why. The short version: as a `const` this branch could not be reached by any test, and it
// is the branch the phone actually runs.

// Resolved once, lazily: importing `@tauri-apps/api` at module scope would pull it into the
// web bundle, which never uses it.
let tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;
/// Call a **real Tauri command** on the shell, and parse its reply.
///
/// Every command the mobile shell exposes returns a JSON *string*, so parsing belongs here rather
/// than in each caller — forgetting it hands back a `string` that type-checks as whatever was
/// asked for and then fails at the first property access (`meta.assets[0]` → "Cannot read
/// properties of undefined").
async function shellInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  if (!tauriInvoke) {
    const core = await import('@tauri-apps/api/core');
    tauriInvoke = core.invoke;
  }
  const text = (await tauriInvoke(cmd, args)) as string;
  return (text ? JSON.parse(text) : undefined) as T;
}

/// The ordinary path: **one Rust command, `fm`**, taking a dispatch name and the same JSON body
/// the HTTP path posts, because the wire contract already is "name plus JSON". The shell forwards
/// it straight to `fm_app::dispatch`, so there is one command surface and not two.
///
/// `fm_ingest` is the single deliberate exception — it exists because Android cannot carry bytes
/// through `dispatch`'s argument JSON, so it is a real second command and is called with
/// `shellInvoke` directly. Sending it through here made it `dispatch("fm_ingest")`, which came
/// back "unknown command: fm_ingest" — the dispatcher rightly saying it has no such thing.
function nativeInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  return shellInvoke<T>('fm', { cmd, args });
}

async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isPhone()) {
    return nativeInvoke<T>(cmd, args);
  }
  if (import.meta.env.PROD) {
    return http<T>(cmd, args);
  }
  return mock.handle<T>(cmd, args);
}

/// Set once any command comes back `401`. Read by `App.svelte` to swap the whole app for the
/// pairing screen — see `http` below for why this is a flag and not a thrown error.
export let needsPairing = false;
let onPairingNeeded: (() => void) | undefined;

/// Let the shell hear about it immediately rather than on its next poll.
export function onNeedsPairing(f: () => void): void {
  onPairingNeeded = f;
}

/// After a successful pairing the cookie exists, so the flag has to be cleared or the screen
/// would persist over a working app.
export function clearPairingFlag(): void {
  needsPairing = false;
}

// POST /api/<cmd> with camelCase args (fm-serve maps them to the Rust snake_case
// params). resolve_asset returns raw bytes (an ArrayBuffer); void commands return
// an empty body.
/// **This tab's identity, for the shutdown watchdog.** In memory only, never stored: a persisted id
/// would make two tabs of the same browser one tab, and closing either would read as closing both.
const TAB_ID = (() => {
  try {
    return crypto.randomUUID();
  } catch {
    // Older WebViews, and any context without a secure origin.
    return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }
})();

/// **Tell the server this tab is going.** The watchdog otherwise waits 90 seconds before giving up
/// on a tab it can no longer hear — long on purpose, because a backgrounded tab's timers are
/// throttled to about once a minute and killing the app under someone still using it is the worse
/// failure. A tab that is *closing* can simply say so, which is better evidence than silence.
///
/// `sendBeacon` because it survives the page going away, where `fetch` is cancelled. It cannot set
/// headers, so the id travels in the query.
export function sayGoodbye(): void {
  try {
    navigator.sendBeacon(`/api/bye?tab=${encodeURIComponent(TAB_ID)}`);
  } catch {
    /* best effort: the 90-second window is still there behind this */
  }
}

async function http<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const res = await fetch(`/api/${cmd}`, {
    method: 'POST',
    // The tab id rides every command, which is how the server learns this tab exists at all — and
    // how a reload re-registers before the goodbye it just sent can matter.
    headers: { 'content-type': 'application/json', 'x-formicaria-tab': TAB_ID },
    body: JSON.stringify(args),
  });
  // **401 is the one status with a meaning rather than a message**: this device has not paired,
  // or its token was revoked. It is raised as a flag rather than thrown to each caller because
  // *every* command gets it at once — the answer is a whole screen, not an error banner on
  // whichever query happened to fire first.
  if (res.status === 401) {
    needsPairing = true;
    onPairingNeeded?.();
  }
  if (!res.ok) throw new Error((await res.text()) || res.statusText);
  if (cmd === 'resolve_asset') return (await res.arrayBuffer()) as T;
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

// Argument keys are camelCase; fm-serve maps them to the Rust snake_case params
// (groupBy -> group_by). The single-word ones pass through unchanged.
export const getBoard = (groupBy: string) => invoke<Board>('board', { groupBy });
export const getAgenda = () => invoke<ObjectMeta[]>('agenda');
export const getNote = (id: string) => invoke<NoteDetail | null>('get', { id });
// `vault` is the audience the new note joins — empty means the default vault, an
// unknown name is refused server-side (the create-side twin of `ingestFile`).
export const capture = (body: string, vault = '') =>
  invoke<ObjectMeta>('capture', { body, vault });
export const setProperty = (id: string, key: string, value: string) =>
  invoke<void>('set_property', { id, key, value });
// `base` is the **`version`** — the content hash of the body you last saw (`ObjectMeta.version`),
// *not* the `updated` stamp. The new one comes back, and you hold it for the next write. That
// round trip is the **lost-update guard**: an editor open across someone else's pull would
// otherwise save its pre-merge text straight over the merge. Send `''` to opt out (nothing in
// the app does).
//
// This comment used to say `updated`, and two conflict handlers in `NotePanel.svelte` believed
// it. A stamp never equals a hash, so once they had conflicted they conflicted forever and the
// note stopped reaching disk entirely. The field is `version` — keep it that way here, because
// this line is what callers read.
export const updateBody = (id: string, body: string, base = '') =>
  invoke<string>('update_body', { id, body, base });
// Destructive: unlink the note's file + index rows. Named `deleteNote` because
// `delete` is a reserved word; the wire command is still `delete`.
export const deleteNote = (id: string) => invoke<void>('delete', { id });
export const search = (query: string) => invoke<ObjectMeta[]>('search', { query });
export const recent = () => invoke<ObjectMeta[]>('recent');
// The collaboration read-model: who last edited each note, and when, from each vault's git log,
// aggregated newest-first. One command behind the "edited by" labels, the activity stream, and
// the contributor filter — git already knows, we only read.
export const activity = () => invoke<EditEvent[]>('activity');

// Discussion, as notes: one message = one file, ULID-named, in the vault of the note it is
// about. There is no `vault` argument — a reply joins the target's audience, which is not a
// choice to leave to a UI.
//
// `reply` takes the note OR another message: replying to a message re-roots to the same
// discussion server-side, so the natural "reply to this comment" gesture cannot create a
// thread nothing can reach.
export const reply = (id: string, body: string) =>
  invoke<ObjectMeta>('reply', { id, body });
export const thread = (id: string) => invoke<ThreadView>('thread', { id });

// **Every thread's message count, in one pass.** The feed needs to say "3 replies" per post, and
// `thread()` is a whole-corpus read (`fm-app/tests/perf.rs`) — so one per row would be thirty
// corpus scans behind one mutex, each parking the phone's JS thread. This returns `{id, count}`
// for every root at once, which is why a count is affordable and opening one is the thing that
// costs. Already what the study agent watches, so it covers note comment threads too.
export const threadRoots = () => invoke<{ id: string; count: number }[]>('thread_roots');

// A first-class discussion: a note that is the root of its own thread (`thread_of` points at
// itself). Created explicitly — `create_discussion` writes the self-anchor, which `set_property`
// refuses, the same way `reply` (not `set_property`) writes a message's pointers. `vault` is the
// audience it joins; empty means the default.
export const createDiscussion = (title: string, vault = '') =>
  invoke<ObjectMeta>('create_discussion', { title, vault });

/** Every first-class discussion across vaults, most-recently-active first — the Discussions view's
 *  feed. Each carries its root note (title + vault), a message count, and the participants (git
 *  authorship). Comment threads hanging off an ordinary note are deliberately not here — those stay
 *  with their note. */
export const discussions = () => invoke<DiscussionSummary[]>('discussions');

/** Notes that link **to** `id` — "what links here" (a `note:` mention or an `![](note:id)` embed).
 *  Derived by scanning bodies, not indexed; newest-updated first. */
export const backlinks = (id: string) => invoke<ObjectMeta[]>('backlinks', { id });

/** Every note tagged `template` — the "New from template" list, most-recently-touched first. A
 *  template is just a tagged note; tag one to make it a starting point, untag to unmake it. */
export const templates = () => invoke<ObjectMeta[]>('templates');

/** **Stop showing me this vault.** Unregisters it: dropped from the live set and from the vault
 *  list on disk. **It never deletes a file** — "forget" and "destroy" are different verbs and only
 *  one is reversible, so the worst case of a mistaken click is retyping a path. The answer says how
 *  many notes were left behind and where, so "removed" cannot be read as "erased".
 *
 *  The fourth verb the vault list needed: three commands created a vault and none removed one, which
 *  on the phone left an auto-created empty vault nobody could get rid of from inside the app. */
export const forgetVault = (name: string) =>
  invoke<{ forgotten: string; path: string; notes: number; remote: string | null; vaults: VaultInfo[] }>(
    'forget_vault',
    { name },
  );

/** Notes that came back from a merge in conflict, **each with its kind**.
 *
 *  Git is asked first (it is what actually blocks every commit in the vault, and the only thing that
 *  knows *how* the two sides disagreed); the body-marker scan is unioned in for notes whose index
 *  entry git has settled but whose text still carries markers. `has_markers === false` means editing
 *  the note is not a resolution — see `resolveConflict`. */
export const conflicts = () => invoke<ConflictInfo[]>('conflicts');

/** Resolve a conflict by keeping a side. **The only resolution that exists for a delete/modify**:
 *  one side has no file, so there is no text to fix. Finishes the merge when it was the last one —
 *  otherwise `MERGE_HEAD` would stand with nothing left to resolve, and a standing `MERGE_HEAD` is
 *  exactly what makes every commit in the vault refuse. */
export const resolveConflict = (vault: string, path: string, keep: 'theirs' | 'mine' | 'edited') =>
  invoke<{ resolved: string }>('resolve_conflict', { vault, path, keep });

/** Notes that are byte-for-byte the same note, grouped. A read: it names what pruning *would*
 *  remove and changes nothing. Grouped by body, because copies differ in `id` and `created`. */
export const duplicates = () => invoke<DuplicateFamily[]>('duplicates');

/** Remove the extra copies in one vault, keeping the oldest of each family.
 *
 *  **Refused while any copy is still outside git history**, because deleting an untracked note is
 *  unrecoverable while deleting a tracked one is a `git checkout` away. Record first, prune second —
 *  the backend enforces that order rather than trusting a caller to remember it. */
export const pruneDuplicates = (vault: string) =>
  invoke<{ removed: number; kept: number }>('prune_duplicates', { vault });

/** Notes on disk that git does not have, per vault — what the app forgot it wrote.
 *
 *  `commit_all` stages only the paths the app remembers writing, and that memory dies with the
 *  process, so every note written before the last restart was permanently unstageable and nothing
 *  said so. This is how it gets said. */
export const unrecorded = () => invoke<Unrecorded[]>('unrecorded');

/** Record them. Explicit, never on a timer: it stages files the app does not remember writing, which
 *  is a decision for a person rather than a five-second debounce. */
export const recordUnrecorded = (vault: string) =>
  invoke<{ committed: boolean; notes: number; reason?: string }>('record_unrecorded', { vault });

/** Every open proposal across vaults — the notes carrying a well-formed `proposes: branch:<name>`,
 *  newest first. The Collaboration surface's feed.
 *
 *  A store query like `recent`, **not** a per-vault git read: it lists the proposal *notes* that
 *  exist. Whether each branch is still open, merged, or gone is derived from git elsewhere — a
 *  proposal naming a branch that no longer resolves is still listed, because the discussion
 *  outlives the branch. Returns plain `ObjectMeta`; branch/base/status badges arrive with the
 *  deferred diff surface. */
export const proposals = () => invoke<ObjectMeta[]>('proposals');

/** The local study-assistant on/off setting — a per-device *launcher* setting (served by fm-serve,
 *  not a vault command). When on, the agent (a small local model that reads your notes and answers
 *  in discussions) auto-starts with formicaria and stops when you close it; off is pure, super-light
 *  formicaria. Takes effect at the next launch. */
/** `enabled` is the stored preference; **`installed` is whether it can actually run here.** The two
 *  used to be conflated, so the switch reported success on a machine with no assistant at all.
 *
 *  `why` is what to print when it cannot: the causes are different (this OS is not there yet / the
 *  stack was never shipped here / `bash` is missing) and want different answers from the reader, so
 *  a bare "not available" is a dead end. Empty when `installed`.
 *
 *  `transcribe_available` is the **sub**-capability: whisper is a separate runtime, and without it
 *  the audio toggle stored a preference and transcribed nothing. */
export const agentStatus = () =>
  invoke<{
    enabled: boolean;
    transcribe: boolean;
    installed: boolean;
    why: string;
    transcribe_available: boolean;
    /** Not here yet, but downloadable on this platform. False alongside `transcribe_available`
     *  means upstream publishes no build — nothing the user can do. */
    transcribe_fetchable: boolean;
    /** Whether the model and runtime are already here — the difference between "turn it on" and
     *  "download a few gigabytes, then turn it on". */
    provisioned: boolean;
    /** What removing the model would free, in bytes — so the control can name the figure. */
    provisioned_bytes: number;
    /** A first-enable download in flight, or how the last one ended; null when idle. `total` is
     *  null where the server sends no length, and the line must then say bytes rather than invent
     *  a percentage. */
    provisioning: {
      stage: 'runtime' | 'model' | 'projector' | 'ready' | 'failed';
      done: number;
      total: number | null;
      error: string | null;
    } | null;
  }>('agent_status');
/** Turn the assistant on or off.
 *
 *  `model` and `vision` matter only on a first enable, when nothing is provisioned yet: they say
 *  which catalogued model to fetch and whether to also fetch the projector that lets it read
 *  images. Both are ignored once the stack is on the machine. */
export const setAgent = (enabled: boolean, model?: string, vision = false) =>
  invoke<{ ok: boolean }>('set_agent', { enabled, model, vision });

/** Delete the downloaded model and runtime, freeing the disk they use. Turns the assistant off
 *  first — deleting files under a running model leaves it serving from unlinked inodes. Answers how
 *  many bytes it freed; refuses (409) on a source checkout, where those files are the developer's. */
export const removeAgentModel = () =>
  invoke<{ freed: number }>('remove_agent_model');

/** What the first-enable screen offers: every catalogued model, its download size and licence. */
export const agentModels = () =>
  invoke<
    {
      name: string;
      bytes: number | null;
      license: string | null;
      vision: boolean;
      mmproj_bytes: number | null;
      default: boolean;
    }[]
  >('agent_models');
/** The "Audio transcription" sub-setting: when on, the assistant loads a local whisper runtime so
 *  `/transcribe` (and the Transcribe-audio action) work. Like the on/off above, it takes effect at the
 *  next assistant start, and needs the runtime staged (`pixi run fetch-whisper`). */
export const setTranscribe = (transcribe: boolean) => invoke<{ ok: boolean }>('set_transcribe', { transcribe });

/** What the study agent is doing *right now* in a discussion — a transient, in-memory status served
 *  by fm-serve (not a vault command), so the discussion view can show a live "working…" wheel with
 *  the current pipeline stage (`searching the web`, `thinking`) and elapsed seconds, and hide it the
 *  instant the reply lands or a timeout clears it. Degrades to inactive where the endpoint is absent
 *  (the mobile shell has no agent yet), so it never throws in a caller's poll loop. */
export type AgentActivity = {
  active: boolean;
  stage?: string;
  question?: string;
  elapsed_secs?: number;
};
export const agentActivity = (discussion: string) =>
  invoke<AgentActivity>('agent_activity_poll', { discussion }).catch(
    () => ({ active: false }) as AgentActivity,
  );

/** The study agents that are alive right now (by `@name`), served by fm-serve's presence registry.
 *  Powers the `@`-picker and the "assistant is offline" warning. Empty where the endpoint is absent
 *  or nobody is running, so callers never throw. */
export const onlineAgents = () =>
  invoke<{ agents: string[] }>('agents', {})
    .then((r) => r.agents ?? [])
    .catch(() => [] as string[]);

/** Propose a change to an existing note. The change lands on a `proposal/<id>` branch (never `main`)
 *  and a proposal note records it for the Collaboration view; a person reviews and merges it. It is
 *  **refused, never truncated**, when it exceeds the target vault's guardrails (`vault.json` →
 *  `proposals`: max files/size per proposal, max open count/size per vault). Returns the proposal
 *  note. This is also the single write path the study agent uses — it can only ever edit one note. */
/** Record what may be done with a vault's review record. Two independent answers — omitting one
 *  leaves it as it was, so a UI can offer them as separate switches without either implying the
 *  other. Host-bound: a paired device may not answer this for the machine that owns the vault. */
export const setSupervision = (vault: string, next: { collect?: boolean; publish?: boolean }) =>
  invoke<VaultInfo[]>('set_supervision', { vault, ...next });

export const createProposal = (id: string, body: string, why?: string, kind?: string) =>
  invoke<ObjectMeta>('create_proposal', { id, body, why, kind });

/** Record that a proposal was actually put in front of a person.
 *
 *  Without it, "the reviewer read this and left it" and "nobody ever opened it" are the same
 *  absence — and they mean opposite things about the model's output. Idempotent and best-effort:
 *  the first viewing is the one that means anything, and failing to note it must never break the
 *  screen it is reporting about. */
export const proposalShown = (id: string) => invoke<void>('proposal_shown', { id });

/** The read half of review: a proposal's change as a unified diff against `main`, plus the files it
 *  touches. `exists: false` (empty diff) is the normal answer for a proposal whose branch is merged
 *  or gone — the note outlives its branch, so this reports "nothing to show", never an error. */
export const proposalDiff = (id: string) => invoke<ProposalDiff>('proposal_diff', { id });

/** The id of a note's current OPEN proposal (its PR), or null. Lets a note's own view show its PR —
 *  the diff + Accept/Reject — beside the discussion, since a proposal has no separate discussion. */
export const proposalFor = (noteId: string) => invoke<string | null>('proposal_for', { id: noteId });

/** The proposed note behind a proposal — host id, title, and the proposed body on the branch — so the
 *  review can SHOW and EDIT it. Saving an edit goes back through `create_proposal` (rebasing the branch
 *  on the current note), which also resolves a stale conflict. Null when the branch is gone. */
export const proposalContent = (id: string) =>
  invoke<ProposalContent | null>('proposal_content', { id });

/** The write half of review — accept a proposal by merging its branch into `main`. The GUI's merge
 *  button, since a UI-only user has no `git merge`. `'merged'` = landed on main (branch deleted);
 *  `'conflicted'` = could not merge cleanly, so it was aborted and main left untouched; `'already_gone'`
 *  = the branch was already merged/deleted (pressing Accept twice). Fail-closed: never leaves a
 *  half-merged tree. */
export const acceptProposal = (id: string) =>
  invoke<{ outcome: 'merged' | 'conflicted' | 'already_gone' }>('accept_proposal', { id });

/** Reject a proposal — the GUI's "Reject" button. Deletes the proposal's branch (local + remote) and
 *  the proposal note; **main is never touched**, so the worst case of a mistaken reject is a proposal
 *  you re-run. The safe inverse of Accept. */
/** `why` is the reviewer's own sentence about what they changed, and it is OPTIONAL on purpose: a
 *  required prompt produces satisficing rather than reasons, and a skipped one is itself a signal.
 *  It becomes the commit body on a proposal, and — since a rejection writes no commit and the
 *  rejected text never reaches `main` — the proposal note's only record of why on a reject. */
export const rejectProposal = (id: string, why?: string) =>
  invoke<void>('reject_proposal', { id, why });

/** Notes nothing has touched since `since` (a git `--since` value), oldest first.
 *
 *  **Derived from git, stored nowhere** — there is no `stale:` property to keep true, and the
 *  threshold is this argument rather than a setting on disk. A vault with no history is
 *  skipped rather than reported as entirely stale: "no evidence" and "old" are different
 *  answers. */
export const staleNotes = (since = '') => invoke<ObjectMeta[]>('stale', { since });

// Copy a note into another vault. Restrictive by default: only the prose travels —
// links & attached files are stripped, so the copy can never point at anything outside
// its new audience. `withAssets` opts in to carrying the first-degree files *into* the
// target so it is self-contained. Returns the new note's meta plus the blob hashes this
// copy newly wrote, so an Undo (`uncopyNote`) can take exactly those back.
export interface CopyResult {
  meta: ObjectMeta;
  new_blobs: string[];
  replaced: number; // prior copies of the same source this one replaced in the target
}
export const copyNote = (id: string, vault: string, withAssets: boolean) =>
  invoke<CopyResult>('copy_note', { id, vault, with_assets: withAssets });
// Does the target vault already hold a copy of this note? Drives the "will replace" warning.
export const copyStatus = (id: string, vault: string) =>
  invoke<boolean>('copy_status', { id, vault });
export const uncopyNote = (id: string, vault: string, blobs: string[]) =>
  invoke<void>('uncopy_note', { id, vault, blobs });

// Asset bytes for the webview. `kind` picks the derived thumbnail or the full
// blob; the caller wraps the ArrayBuffer in an object URL. Returns null in the
// browser/test mock (no vault), so callers fall back to the missing placeholder.
//
// For the *full* blob prefer `assetUrl` below — this path holds the entire file in
// memory twice (once here, once in the object URL) and cannot seek.
export const resolveAsset = (reference: string, kind: 'full' | 'thumb') =>
  invoke<ArrayBuffer | null>('resolve_asset', { reference, kind });

// Whether this backend can stream a blob from a URL rather than holding it in memory.
//
// True in two different ways, and that is the point: `fm-serve` streams over HTTP, and the
// mobile shell streams over the `fmblob://` URI scheme it registers. The in-memory mock
// (`pnpm dev`, Vitest) has neither and keeps the bytes-and-object-URL path.
//
// **This was silently wrong on the phone.** A Tauri build is `import.meta.env.PROD`, so it
// claimed to stream and then pointed at `/api/blob/…`, a route that only exists in the desktop
// server — every image failed. Same trap as the `invoke` transport switch above, in the same
// file, for the same reason: PROD is not a statement about which backend is present.
//
// So it names the two streaming backends explicitly rather than leaning on `PROD` to imply them,
// and it is a function for the same reason `isPhone` is: as a `const` this read `false` under
// Vitest even in phone mode, so a phone test would have silently exercised the mock's
// object-URL path instead of the `fmblob://` one the device uses.
export function streamsBlobs(): boolean {
  return isPhone() || import.meta.env.PROD;
}

// "A tab is still here" — and nothing else. Not a command: it takes no lock, reads no
// files, and never reaches `fm_app::dispatch`, because the auto-shutdown watchdog is a
// property of this server rather than of the app. Kept separate from `ping` so a hidden
// tab can stay alive without making the server reindex a vault it is not looking at.
// It also carries the **share capability** back, which is why it now has a return value. That
// report has to arrive on a timer — a listener can fail, and a network can stop carrying packets,
// long after the settings screen was last looked at — and this is already the transport's own
// beat. `ping` belongs to the command surface, which knows nothing about sharing.
export async function alive(): Promise<ShareStatus | null> {
  if (!import.meta.env.PROD) return null;
  const r = await fetch('/api/alive', { method: 'POST' }).catch(() => null);
  if (!r?.ok) return null;
  return (await r.json().catch(() => null)) as ShareStatus | null;
}

// ---- Sharing with a nearby device -----------------------------------------------------------
//
// Transport routes, not commands: `dispatch` is shared with `fm-cli` and the phone, and *who is
// calling* is a property of the connection that neither of them has. All of these except `pair`
// are refused unless the request came from the computer itself.

/// Mint a pairing code for the chosen vaults. **A device is paired to audiences, never to the
/// machine**, so this refuses an empty list rather than quietly granting everything.
export const shareCode = (vaults: string[]) =>
  post<{ code: string; expires_in: number }>('/api/share_code', { vaults });

export const setShare = (enabled: boolean) =>
  post<{ enabled: boolean; restart_required: boolean }>('/api/set_share', { enabled });

export const revokeDevices = () => post<{ devices: number }>('/api/revoke_devices', {});

/// Redeem a code. The token comes back as an `HttpOnly` cookie, so nothing here ever sees it —
/// which is the point: a script that could read it could exfiltrate it, and note bodies arrive
/// from collaborators.
export const pairDevice = (code: string, name: string) =>
  post<{ vaults: string[] }>('/api/pair', { code, name });

/// These sit beside `invoke` rather than going through it: they are not commands, so they have no
/// place in the command dispatcher's error handling or its argument conventions.
async function post<T>(path: string, args: unknown): Promise<T> {
  const r = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!r.ok) throw new Error((await r.text()) || r.statusText);
  return (await r.json()) as T;
}

// A URL a media element can point at directly, so the browser fetches only the
// bytes it needs. `<video>` seeking becomes a `Range` request instead of a
// whole-file download, and nothing has to be revoked afterwards.

/** The derivation above, wired to whatever Tauri put on `window`. */
function assetBase(): string {
  const internals = (window as unknown as {
    __TAURI_INTERNALS__?: { convertFileSrc?: (path: string, protocol: string) => string };
  }).__TAURI_INTERNALS__;
  return blobBase(internals?.convertFileSrc);
}

/** The URL a blob is served from, with any **anchor fragment kept as a fragment**.
 *
 *  `asset:sha256-…#page=4` points at a place in a PDF, and `#page=N` is the standard PDF open
 *  parameter every real viewer honours. Percent-encoding the whole reference into the path — which
 *  is what this did — buried the `#` where no browser could act on it, so an anchored link opened
 *  the document at page one. The reference is split, the *blob* half encoded, and the fragment put
 *  back where a URL fragment belongs. */
export const assetUrl = (reference: string, kind: 'full' | 'thumb' = 'full') => {
  const hash = reference.indexOf('#');
  const blob = hash === -1 ? reference : reference.slice(0, hash);
  const fragment = hash === -1 ? '' : reference.slice(hash);
  const base = !isPhone() ? '/api/blob/' : assetBase();
  // **The query goes before the fragment**, which is the whole reason this is built by hand rather
  // than concatenated: a `#page=3` has to stay a fragment for the PDF viewer to act on it, and
  // `?kind=thumb#page=3` is the only order a URL parser reads correctly.
  const query = kind === 'thumb' ? '?kind=thumb' : '';
  return `${base}${encodeURIComponent(blob)}${query}${fragment}`;
};

/// Set which attachments this vault pushes with its notes. `max` is a size a person writes
/// ("2MB"), or empty to turn it off. Returns the refreshed vault list, so the caller never has to
/// guess what was actually stored — the backend normalises the size and it comes back formatted.
export const setGitAssetsMax = (vault: string, max: string) =>
  invoke<VaultInfo[]>('set_git_assets_max', { vault, max });

export const assetStatus = (reference: string) =>
  invoke<AssetStatus>('asset_status', { reference });
export const openExternal = (reference: string) =>
  invoke<void>('open_external', { reference });

// Ingest an uploaded file (drag-drop / picker): stores a content-addressed blob,
// extracts text, creates an asset note, and returns its meta so the editor can
// insert a reference. Raw bytes go in the POST body; the name and vault ride the query.
//
// `vault` is the audience the file joins, and it matters: a PDF dropped onto a lab note
// belongs in the lab vault, beside the notes that reference it and inside the boundary
// its readers already have. Empty means the default vault.
/// The ceiling on a single attachment **on Android only**, where the bytes ride inside a JSON
/// string. Base64 inflates by a third and the payload is copied a few times between the page and
/// Rust, so a large video is not slow here — it fails, or takes the app down with it.
///
/// **It was 48 MB, and that number was arrived at by counting the wrong thing.** The old note
/// reasoned that 48 MB sits "well inside what a WebView will serialise", which is true of the
/// string and false of the operation. Counting the copies a file actually makes on the way in:
/// the `File` itself, the `readAsDataURL` result (~1.37×), Tauri's `JSON.stringify` of the whole
/// message (~1.37×), the JS→Java string marshal (~2.7×, UTF-16), wry's `get_string` and
/// `to_string_lossy` (~1.37× each), `serde_json`'s parse into a `Value` and then into the `data`
/// argument (~1.37× each), and finally the decoded `Vec<u8>`. That is roughly **ten copies**, so a
/// 12 MP photo peaks around 150 MB and the old ceiling permitted a peak near 600 MB — which is not
/// a slow attach, it is `onRenderProcessGone` and the app disappearing with no message
/// (`known-issues.md`: the framework default kills the process and nothing here overrides it).
///
/// **16 MB keeps every photo a phone takes** — a 12 MP JPEG is ~4 MB, a 48 MP one ~12 MB — and
/// drops the peak to roughly a third. Chunking the *encode* would not have helped: it removes one
/// copy of the ten, all the others being on the transport. The real fix is chunked **ingest**
/// (`fm_ingest_chunk`/`fm_ingest_finish` over `BlobStore::put_file`, which already streams and
/// hashes in 64 KB chunks), which bounds the transient regardless of file size and lifts the video
/// refusal. That is a transport change, deliberately not smuggled in beside a bug fix; this number
/// buys the headroom to do it properly. Recorded in `known-issues.md`.
const MAX_INGEST = 16 * 1024 * 1024;

/// How much of a file goes in one chunked message.
///
/// **Well under `MAX_INGEST`, and that is the point.** The ceiling above is where base64 in a
/// JSON argument stops fitting; a slice this size is nowhere near it, so the transient peak stops
/// scaling with the file. 2 MB is ~2.7 MB base64 — small enough that a dozen live copies are
/// still nothing, large enough that a 200 MB video is a hundred messages rather than thousands.
///
/// Kept below `fm_core::chunked::MAX_CHUNK` (4 MB), which refuses anything larger. That limit
/// exists precisely so a frontend cannot reintroduce the problem by calling the whole file
/// "chunk 0".
const CHUNK = 2 * 1024 * 1024;

/// A `Blob` slice as standard base64, without the `data:` prefix.
///
/// Same `FileReader` reasoning as `base64` below: spreading a multi-megabyte array into
/// `String.fromCharCode(...)` throws `RangeError` on exactly the sizes worth sending.
function base64Slice(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onerror = () => reject(new Error('could not read part of the file'));
    r.onload = () => {
      const t = String(r.result);
      const comma = t.indexOf(',');
      resolve(comma >= 0 ? t.slice(comma + 1) : t);
    };
    r.readAsDataURL(blob);
  });
}

/// An id for one upload. Letters, digits and `-` only — `fm_core::chunked` validates it as a path
/// segment and **refuses** anything else rather than rewriting it, so a generated id must already
/// be in that alphabet.
function uploadId(): string {
  const rand = Math.random().toString(36).slice(2, 10);
  return `u${Date.now().toString(36)}-${rand}`;
}

/// A `File` as standard base64, without the `data:` prefix.
///
/// `FileReader` rather than `btoa(String.fromCharCode(...bytes))`: spreading a multi-megabyte
/// array into a call blows the argument limit and throws `RangeError` on exactly the files worth
/// attaching. The browser does this conversion natively and in one pass.
function base64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onerror = () => reject(new Error(`could not read ${file.name}`));
    r.onload = () => {
      const s = String(r.result);
      const comma = s.indexOf(',');
      // `data:<mime>;base64,<payload>` — everything after the first comma is the payload.
      resolve(comma >= 0 ? s.slice(comma + 1) : s);
    };
    r.readAsDataURL(file);
  });
}

export async function ingestFile(file: File, vault = ''): Promise<ObjectMeta> {
  // **The same POST the desktop makes, to a different base.** Tauri's raw IPC body does not
  // exist on Android — its own docs: "On Android, InvokeBody::Raw is not supported." Sending a
  // photo as JSON would mean base64, a third larger and copied several times. The shell's
  // `fmblob` protocol handler receives a request body as bytes, so this is an ordinary `fetch`
  // with the File as the body, exactly as the browser path below does.
  if (isPhone()) {
    // **Base64 over the IPC command, because Android has no other door.**
    //
    // This was a `fetch` POST to the `fmblob://` handler, which is correct-looking and silently
    // sends nothing: wry intercepts through `WebViewClient.shouldInterceptRequest`, whose
    // `WebResourceRequest` exposes the URL, method and headers — **and no body**. Android has no
    // accessor for one. So every photo arrived as zero bytes, `ingest` hashed the empty string,
    // and every capture produced the same reference. Tauri's raw IPC body is not available here
    // either (its docs: "On Android, InvokeBody::Raw is not supported"), which leaves JSON, which
    // means base64.
    //
    // The cost is real — about a third more bytes, and a few copies — and it is the price of the
    // media arriving at all.
    // **Over the single-shot ceiling, the file is sliced instead of refused** (2026-09-04).
    //
    // The ceiling was never a judgement about attachment size: it is where base64 in a JSON
    // argument, copied several times between here and Rust, stops fitting on a phone —
    // `outstanding.md` §1.3 called it *"a memory limit wearing a size limit's clothes"*, and it
    // is what refused video. Chunking bounds the transient at one slice whatever the file
    // weighs, and the blob is stored from the assembled file by `BlobStore::put_file`, which
    // already streams and hashes in 64 KB reads.
    //
    // **Small files still take the single-shot path.** One message is cheaper than five, and the
    // path that carries every photo anyone has ever taken with this app should not be rerouted
    // through new code for no gain.
    if (file.size <= MAX_INGEST) {
      const data = await base64(file);
      return shellInvoke<ObjectMeta>('fm_ingest', { name: file.name, vault, data });
    }
    const session = uploadId();
    try {
      let seq = 0;
      for (let at = 0; at < file.size; at += CHUNK) {
        const data = await base64Slice(file.slice(at, Math.min(at + CHUNK, file.size)));
        // Sequential and awaited, deliberately. The backend checks `seq` and refuses a gap, so
        // firing these in parallel would race them into a refusal — and the point of chunking is
        // to hold one slice at a time, which parallel sends undo.
        await shellInvoke<string>('fm_ingest_chunk', { session, seq, vault, data });
        seq += 1;
      }
      return shellInvoke<ObjectMeta>('fm_ingest_finish', { session, name: file.name, vault });
    } catch (e) {
      // **Reclaim the bytes now.** A failed upload on a phone otherwise leaves its slices in
      // app-private storage until the next boot sweep, which on a device someone leaves running
      // is a long time to hold a partial video. Best-effort: the original error is what the user
      // needs to see, so a failed cleanup must not replace it.
      await shellInvoke<void>('fm_ingest_cancel', { session, vault }).catch(() => {});
      throw e;
    }
  }
  if (import.meta.env.PROD) {
    const q = `name=${encodeURIComponent(file.name)}&vault=${encodeURIComponent(vault)}`;
    const res = await fetch(`/api/ingest?${q}`, {
      method: 'POST',
      body: file,
    });
    if (!res.ok) throw new Error((await res.text()) || res.statusText);
    return res.json();
  }
  return mock.handle<ObjectMeta>('ingest', { name: file.name, vault });
}

// Durability, in two tiers. Light: commit the vault's notes to its own git repo
// (returns true if a commit was made) and push them to its remote — text only,
// no media. Heavy: snapshot the whole vault, blobs included, to restic (repo +
// password from the env). `backupStatus` reports what each tier could do right
// now, so the panel promises only what it can deliver.
// Git is per vault — one vault is one repo, one remote, one collaborator list — so
// every command below names the vault it acts on. An empty/absent name means the
// default (the first configured vault), which is what a single-vault install always is.
export const commit = (message: string, vault = '') =>
  invoke<CommitResult>('commit', { message, vault });
/** Snapshot one vault's media into *its own* restic repo. Per vault because a restic
 *  repo is per repository — there is no one destination a set of vaults could share. */
export const backup = (vault = '') => invoke<void>('backup', { vault });
export const backupStatus = () => invoke<BackupStatus>('backup_status');

/** When this vault's media was last snapshotted, and what that snapshot covered.
 *
 *  **Deliberately not part of `backupStatus`.** That one polls every 45 s and already spawns a
 *  process per vault; this spawns another and may be a network round trip, so it is asked when
 *  someone opens the panel rather than on a timer. `unavailable` carries the reason when there is
 *  no answer to be had — which is not the same as `id: null`, meaning the repo opened and has
 *  never been written to. */
export const backupLatest = (vault = '') =>
  invoke<LatestBackup>('backup_latest', { vault });

/** Point one vault's media backup at a restic repository — a path, or an `s3:`/`sftp:` URL.
 *
 *  Per vault, because a restic repo is per repository. An empty `repo` clears it, which is how a
 *  user says "this vault's media stays here" without editing a file. Answers the fresh
 *  `backup_status`, so the panel's promise is the backend's, not one it computed for itself. */
export const setResticRepo = (repo: string, vault = '') =>
  invoke<BackupStatus>('set_restic_repo', { repo, vault });

/** The password that unlocks every restic repository this machine writes to.
 *
 *  **One for all of them** — a per-vault password would multiply the places a secret lives. Kept
 *  `0600` in formicaria's own config directory and never in `vaults.json`, which is a file of paths
 *  a user may reasonably open or send someone. Write-only from here: nothing reads it back, and
 *  `backup_status` reports only whether there is one.
 *
 *  **Lose it and the backups are gone.** Restic has no recovery for a repository whose password is
 *  missing, which is why the panel says so at the moment it is set. */
export const setResticPassword = (password: string) =>
  invoke<BackupStatus>('set_restic_password', { password });

/** Forget the stored password. The repositories are untouched and still need it. */
export const clearResticPassword = () => invoke<BackupStatus>('clear_restic_password');
/** Point the vault at a remote. `name`/`email` are sent only when the vault has no
 *  identity yet — sharing a vault is what makes the committer name matter, so it is
 *  the one moment worth asking. */
export const setGitRemote = (
  url: string,
  identity?: { name: string; email: string },
  vault = '',
) => invoke<void>('set_git_remote', { url, vault, ...identity });
/** Who is committing, with no remote involved. `setGitRemote` can also set this, but only
 *  alongside a URL — and git refuses to commit *anything* without a committer, so a user who
 *  never shares still needs it. Asked once on first run; skippable, because notes on disk do
 *  not depend on git having an opinion about who wrote them. */
export const setIdentity = (name: string, email: string, vault = '') =>
  invoke<void>('set_identity', { name, email, vault });
/** Squashes the unpushed commits into one; returns how many were squashed. */
export const push = (message: string, vault = '') => invoke<number>('push', { message, vault });
/** Bring a collaborator's work home. Merges through the `.md` driver, so two people
 *  editing different paragraphs of one note is a non-event; a genuine disagreement
 *  comes back in `conflicts` with the markers in the note's body. */
export const pull = (vault = '') => invoke<PullResult>('pull', { vault });

// Liveness heartbeat. When launched from the desktop icon the server auto-shuts
// down once the tab stops pinging, so closing the tab closes the app. A no-op in
// the dev/test mock backend.
/** Liveness heartbeat, the local poll, and one capability. `changed` is true when the
 *  vault moved on disk under us (a pull, a merge driver, an editor), which the views
 *  cannot see on their own because they are served from the index. `git` says whether this
 *  machine has git at all — **not a dependency, a capability**: the notebook works without
 *  it, only history does not. `restic` is the same kind of claim, and it decides whether
 *  "restore from a backup" is offered at all — Android has no restic and never will, so
 *  there the route is absent rather than present and failing. */
// `since` is the `generation` the last beat handed back — the cursor into "how many times have
// the vaults moved". Hold it and send it every time: `changed` is nothing more than
// `generation > since`, computed server-side.
//
// It is per-client state and it has to be, because `changed` used to be derived from whether
// *this* reindex found drift — and a reindex writes the fresh mtimes back, so the first tab to
// ask consumed the answer and every other client was told "nothing changed" indefinitely. With
// one tab that was a curiosity; with a tab and a tablet it is every edit.
export const ping = (since = 0) =>
  invoke<{
    changed: boolean;
    generation: number;
    git: boolean;
    restic: boolean;
    skipped: SkippedNote[];
    /** Configured vaults that would not open, as `name: why`. A vault that is silently absent is
     *  indistinguishable from data loss, so the heartbeat names it. Almost always empty. */
    unopened_vaults: string[];
  }>('ping', { since });

/** A note the vault could not read, and enough to show it in a list. No path: the backend
 *  resolves that from the same set, so the only files openable this way are ones it just
 *  reported as broken. */
export type SkippedNote = { vault: string; name: string; reason: string };

/** Hand an unreadable note to the OS editor. A desktop convenience only — on the phone there
 *  is no OS editor, so the in-app raw editor (`readSkipped`/`resolveSkipped`) is the real path.
 *  Fails closed if the note has since been fixed, which is what makes a stale panel harmless. */
export const openSkipped = (vault: string, name: string) =>
  invoke<void>('open_skipped', { vault, name });

/** The **raw text** of an unreadable note, for the in-app editor — markers and all. Same
 *  allowlist as `openSkipped`: the path is resolved from the backend's current skipped set,
 *  never the caller. This is what lets a conflict be fixed in-app, including on the phone. */
export const readSkipped = (vault: string, name: string) =>
  invoke<{ text: string }>('read_skipped', { vault, name });

/** Write the user's resolved raw text back over an unreadable note (byte-for-byte, no
 *  side-picking). Returns whether it now parses, so the editor can tell "resolved" from
 *  "still has conflict markers". The note re-enters every view on the next poll. */
export const resolveSkipped = (vault: string, name: string, text: string) =>
  invoke<{ parses: boolean }>('resolve_skipped', { vault, name, text });

/** The audiences that exist. `[]` is the first-run signal — the one answer that means
 *  "nothing else in this app can work yet". */
export const listVaults = () => invoke<VaultInfo[]>('list_vaults');
/** What creating a vault here would do. Called per keystroke; the server owns the verdict. */
export const checkPath = (name: string, path: string) =>
  invoke<PathCheck>('check_path', { name, path });
/** Create, register and open a vault — live, with no restart. Returns the new list. */
export const createVault = (name: string, path: string) =>
  invoke<VaultInfo[]>('create_vault', { name, path });

/** What importing this folder would involve, and whether it can be. Called per keystroke like
 *  `checkPath`, and it answers for the destination too — so the panel never has to combine two
 *  verdicts of its own. `vault` names an existing vault; empty means the new one in `name`/`path`. */
export const checkImport = (source: string, vault = '', name = '', path = '') =>
  invoke<ImportCheck>('check_import', { source, vault, name, path });

/** Convert a Logseq graph or an Obsidian vault into notes. Host-only (it reads a folder on this
 *  machine), and a single blocking call — the preview above is what makes that acceptable, since
 *  it states the size before the button is pressed. */
export const runImport = (
  source: string,
  vault: string,
  name: string,
  path: string,
  stubs: boolean,
) => invoke<ImportReport>('run_import', { source, vault, name, path, stubs });

/** Clone a collaborator's vault and register it. The identity is **required**, not a
 *  courtesy: a shared vault is exactly where committing as the placeholder would attribute
 *  everyone's work to one fake person. Validated before anything is fetched, so a typo
 *  refuses while the disk is still untouched. */
export const cloneVault = (
  name: string,
  path: string,
  url: string,
  gitName: string,
  gitEmail: string,
) => invoke<VaultInfo[]>('clone_vault', { name, path, url, gitName, gitEmail });

/** Can we reach this repo, and if not, why not — asked before a clone commits to a folder.
 *  Never throws: every outcome is a state the form renders, because this runs while the user
 *  is still typing and an error banner per keystroke would be worse than useless. */
export const probeRemote = (url: string) =>
  invoke<RemoteProbe>('probe_remote', { url });

/** Where this machine keeps git credentials, and whether it has one for this URL. */
export const gitAuth = (url = '') => invoke<GitAuth>('git_auth', { url });

/** Give this machine a credential for a private repo.
 *
 *  **Where it lands depends on the platform, and that is deliberate.** With git installed it
 *  goes to git's own credential helper — the platform keychain — and formicaria stores nothing,
 *  so the terminal and every other tool get it too. On a phone there is no helper, so the app
 *  keeps it in its own private storage.
 *
 *  The token is write-only from the UI's side: nothing ever reads it back. */
export const setGitCredential = (url: string, token: string, username = '') =>
  invoke<GitAuth>('set_git_credential', { url, token, username });

/** Forget the token this device holds. Only meaningful where storage is `app`. */
export const clearGitCredential = (url = '') =>
  invoke<GitAuth>('clear_git_credential', { url });

/** Restore a vault from a restic backup and register it — the third way a vault comes into
 *  being, and the one for a machine that is not the machine the vault was on.
 *
 *  **What comes back is notes and media, with no history.** `backup` snapshots the vault's
 *  own directories and deliberately not its root, so `.git` was never in the repo — no
 *  remote, no collaborators, no identity. It is a *recovery*, not a *join*.
 *
 *  No password argument, and there will not be one: it is read from `RESTIC_PASSWORD` on the
 *  server. The app holds no secret of its own and a restore is not the place to start. */
export const restoreVault = (name: string, path: string, repo: string) =>
  invoke<VaultInfo[]>('restore_vault', { name, path, repo });

/** What this installation is configured as. Cheap — no shelling out — so Settings can be
 *  opened freely, unlike `backupStatus` which runs `git ls-remote` per vault. */
export const config = () => invoke<Config>('config');

/** Create a paper note from whatever the user pasted — a BibTeX entry, a DOI, an arXiv id or
 *  link, or a bare title. **Offline**: the core links no HTTP client (the owner's ruling in
 *  `fm-agent-run`'s Cargo.toml), so an identifier is *recognised*, never resolved. */
export const createPaper = (input: string, vault = '') =>
  invoke<ObjectMeta>('create_paper', { input, vault });
/** This note's citation as a BibTeX entry. Server-side so the format has one implementation. */
export const paperBibtex = (id: string) => invoke<string>('paper_bibtex', { id });
/** The user's saved `.view` files, aggregated across vaults. A broken one carries `error`. */
/** Save the arrangement you are looking at as a named view, in the vault's `views/` folder.
 *  Still not a filter editor — the grammar is nine predicates and a UI for it is a query builder —
 *  but one `tag` may narrow it, because "the ones tagged `paper`" is a sentence a person says and
 *  without it the app can offer no filtered view at all. A view whose filter is richer than a
 *  single tag is refused rather than silently flattened. */
export const saveView = (
  name: string,
  view: 'board' | 'agenda' | 'timeline',
  group_by = '',
  vault = '',
  tag = '',
) => invoke<ViewInfo[]>('save_view', { name, view, group_by, vault, tag });
/** Rename a saved view. **Its own command on purpose** — saving under a new name and deleting the
 *  old one slips past the refuse-don't-flatten guard and would silently drop a hand-written filter. */
export const renameView = (from: string, to: string, vault = '') =>
  invoke<ViewInfo[]>('rename_view', { from, to, vault });
/** Remove a saved view. Missing is success — the user asked for it to be gone. */
export const deleteView = (name: string, vault = '') =>
  invoke<ViewInfo[]>('delete_view', { name, vault });
export const listViews = () => invoke<ViewInfo[]>('list_views');

/** Every theme across every vault. Each carries the vault that holds it, which the write commands
 *  below need — a name alone resolves against the default vault and finds nothing. */
export const listThemes = () => invoke<ThemeInfo[]>('list_themes');
/** The CSS of one theme, for the editor and for applying it. */
export const readTheme = (name: string, vault = '') =>
  invoke<string>('read_theme', { name, vault });
/** Write a theme and get the new list back. Refused above 128 KB, before anything is written. */
export const saveTheme = (name: string, css: string, vault = '') =>
  invoke<ThemeInfo[]>('save_theme', { name, css, vault });
/** Rename a theme. Moves the bytes; never rewrites them. */
export const renameTheme = (from: string, to: string, vault = '') =>
  invoke<ThemeInfo[]>('rename_theme', { from, to, vault });
/** Remove a theme. Missing is success — the user asked for it to be gone. */
export const deleteTheme = (name: string, vault = '') =>
  invoke<ThemeInfo[]>('delete_theme', { name, vault });
/** Run one saved view by name — the query is defined server-side; we send only the name. */
export const runView = (name: string) => invoke<ViewResult>('run_view', { name });

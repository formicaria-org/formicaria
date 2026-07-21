import type {
  Config,
  AssetStatus,
  BackupStatus,
  Board,
  CommitResult,
  EditEvent,
  GitAuth,
  DiscussionSummary,
  NoteDetail,
  ObjectMeta,
  PathCheck,
  PullResult,
  ThreadView,
  RemoteProbe,
  VaultInfo,
  ViewInfo,
  ViewResult,
} from './types';
import * as mock from './mock';
import { blobBase } from './blobBase';

// Two backends, one contract:
//   • Production browser (served by `fm-serve`) → the Rust commands over HTTP
//     (POST /api/<cmd>) against the real vault.
//   • `pnpm dev` / tests → an in-memory mock, so the UI stays developable with no
//     backend at all.
// `import.meta.env.PROD` is true only in the built bundle, so dev and tests hit
// the mock while the shipped bundle hits real data over HTTP.
// Is this bundle running inside the Tauri shell rather than a browser? Tauri v2 puts this on
// `window` before any of our code runs.
//
// **The order matters, and it is the trap.** A Tauri build is `import.meta.env.PROD`, so
// without this branch *first* the app would take the HTTP path and `fetch('/api/…')` against a
// server that does not exist on the device. Same bundle, three backends, and the most specific
// one has to win.
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
  if (isTauri) {
    return nativeInvoke<T>(cmd, args);
  }
  if (import.meta.env.PROD) {
    return http<T>(cmd, args);
  }
  return mock.handle<T>(cmd, args);
}

// POST /api/<cmd> with camelCase args (fm-serve maps them to the Rust snake_case
// params). resolve_asset returns raw bytes (an ArrayBuffer); void commands return
// an empty body.
async function http<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const res = await fetch(`/api/${cmd}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
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
// `base` is the `updated` stamp you last saw for this note; the new one comes back, and you
// hold it for the next write. That round trip is the **lost-update guard**: an editor open
// across someone else's pull would otherwise save its pre-merge text straight over the
// merge. Send `''` to opt out (nothing in the app does).
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

/** Notes that came back from a merge in conflict — both versions marked in the body, needing a
 *  human. Derived by scanning for the markers, so the list is always current (resolve one → it drops). */
export const conflicts = () => invoke<ObjectMeta[]>('conflicts');

/** Every open proposal across vaults — the notes carrying a well-formed `proposes: branch:<name>`,
 *  newest first. The Collaboration surface's feed.
 *
 *  A store query like `recent`, **not** a per-vault git read: it lists the proposal *notes* that
 *  exist. Whether each branch is still open, merged, or gone is derived from git elsewhere — a
 *  proposal naming a branch that no longer resolves is still listed, because the discussion
 *  outlives the branch. Returns plain `ObjectMeta`; branch/base/status badges arrive with the
 *  deferred diff surface. */
export const proposals = () => invoke<ObjectMeta[]>('proposals');

/** Propose a change to an existing note. The change lands on a `proposal/<id>` branch (never `main`)
 *  and a proposal note records it for the Collaboration view; a person reviews and merges it. It is
 *  **refused, never truncated**, when it exceeds the target vault's guardrails (`vault.json` →
 *  `proposals`: max files/size per proposal, max open count/size per vault). Returns the proposal
 *  note. This is also the single write path the study agent uses — it can only ever edit one note. */
export const createProposal = (id: string, body: string) =>
  invoke<ObjectMeta>('create_proposal', { id, body });

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
export const streamsBlobs = import.meta.env.PROD;

// "A tab is still here" — and nothing else. Not a command: it takes no lock, reads no
// files, and never reaches `fm_app::dispatch`, because the auto-shutdown watchdog is a
// property of this server rather than of the app. Kept separate from `ping` so a hidden
// tab can stay alive without making the server reindex a vault it is not looking at.
export async function alive(): Promise<void> {
  if (!import.meta.env.PROD) return;
  await fetch('/api/alive', { method: 'POST' }).catch(() => {});
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

export const assetUrl = (reference: string) => {
  if (!isTauri) return `/api/blob/${encodeURIComponent(reference)}`;
  return `${assetBase()}${encodeURIComponent(reference)}`;
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
/// 48 MB is chosen to sit comfortably above any photo a phone takes (a 12 MP JPEG is ~4 MB, a
/// 48 MP one ~12 MB) while staying well inside what a WebView will serialise. Video is the case
/// this does not serve, and saying so plainly beats an out-of-memory crash — a real fix is
/// chunking, which is a larger piece and is recorded as such.
const MAX_INGEST = 48 * 1024 * 1024;

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
  if (isTauri) {
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
    if (file.size > MAX_INGEST) {
      throw new Error(
        `${file.name} is ${Math.round(file.size / 1e6)} MB. On Android a file is carried inside a ` +
          `text message to the app, so ${Math.round(MAX_INGEST / 1e6)} MB is the ceiling — ` +
          `attach it from the desktop, where there is no such limit.`,
      );
    }
    const data = await base64(file);
    return shellInvoke<ObjectMeta>('fm_ingest', { name: file.name, vault, data });
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
/** Point the vault at a remote. `name`/`email` are sent only when the vault has no
 *  identity yet — sharing a vault is what makes the committer name matter, so it is
 *  the one moment worth asking. */
export const setGitRemote = (
  url: string,
  identity?: { name: string; email: string },
  vault = '',
) => invoke<void>('set_git_remote', { url, vault, ...identity });
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
export const ping = () =>
  invoke<{ changed: boolean; git: boolean; restic: boolean; skipped: SkippedNote[] }>('ping');

/** A note the vault could not read, and enough to show it in a list. No path: the backend
 *  resolves that from the same set, so the only files openable this way are ones it just
 *  reported as broken. */
export type SkippedNote = { vault: string; name: string; reason: string };

/** Hand an unreadable note to the OS editor — the one action available for a conflicted
 *  merge, since by definition no editor of ours can parse it. Fails closed if the note has
 *  since been fixed, which is what makes a stale panel harmless. */
export const openSkipped = (vault: string, name: string) =>
  invoke<void>('open_skipped', { vault, name });

/** The audiences that exist. `[]` is the first-run signal — the one answer that means
 *  "nothing else in this app can work yet". */
export const listVaults = () => invoke<VaultInfo[]>('list_vaults');
/** What creating a vault here would do. Called per keystroke; the server owns the verdict. */
export const checkPath = (name: string, path: string) =>
  invoke<PathCheck>('check_path', { name, path });
/** Create, register and open a vault — live, with no restart. Returns the new list. */
export const createVault = (name: string, path: string) =>
  invoke<VaultInfo[]>('create_vault', { name, path });

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

/** The user's saved `.view` files, aggregated across vaults. A broken one carries `error`. */
export const listViews = () => invoke<ViewInfo[]>('list_views');
/** Run one saved view by name — the query is defined server-side; we send only the name. */
export const runView = (name: string) => invoke<ViewResult>('run_view', { name });

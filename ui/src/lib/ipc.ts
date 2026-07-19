import type {
  Config,
  AssetStatus,
  BackupStatus,
  Board,
  EditEvent,
  NoteDetail,
  ObjectMeta,
  PathCheck,
  PullResult,
  VaultInfo,
  ViewInfo,
  ViewResult,
} from './types';
import * as mock from './mock';

// Two backends, one contract:
//   • Production browser (served by `fm-serve`) → the Rust commands over HTTP
//     (POST /api/<cmd>) against the real vault.
//   • `pnpm dev` / tests → an in-memory mock, so the UI stays developable with no
//     backend at all.
// `import.meta.env.PROD` is true only in the built bundle, so dev and tests hit
// the mock while the shipped bundle hits real data over HTTP.
async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
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

// Whether this backend can stream a blob from a URL. Only the real server has an
// HTTP route to stream from; the in-memory mock (`pnpm dev`, Vitest) has no server
// at all, so it keeps the bytes-and-object-URL path.
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
export const assetUrl = (reference: string) => `/api/blob/${encodeURIComponent(reference)}`;
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
export async function ingestFile(file: File, vault = ''): Promise<ObjectMeta> {
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
  invoke<boolean>('commit', { message, vault });
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
 *  it, only history does not. */
export const ping = () =>
  invoke<{ changed: boolean; git: boolean; skipped: SkippedNote[] }>('ping');

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

/** What this installation is configured as. Cheap — no shelling out — so Settings can be
 *  opened freely, unlike `backupStatus` which runs `git ls-remote` per vault. */
export const config = () => invoke<Config>('config');

/** The user's saved `.view` files, aggregated across vaults. A broken one carries `error`. */
export const listViews = () => invoke<ViewInfo[]>('list_views');
/** Run one saved view by name — the query is defined server-side; we send only the name. */
export const runView = (name: string) => invoke<ViewResult>('run_view', { name });

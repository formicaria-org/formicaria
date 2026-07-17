import type { AssetStatus, BackupStatus, Board, NoteDetail, ObjectMeta, PullResult } from './types';
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
export const capture = (body: string) => invoke<ObjectMeta>('capture', { body });
export const setProperty = (id: string, key: string, value: string) =>
  invoke<void>('set_property', { id, key, value });
export const updateBody = (id: string, body: string) =>
  invoke<void>('update_body', { id, body });
// Destructive: unlink the note's file + index rows. Named `deleteNote` because
// `delete` is a reserved word; the wire command is still `delete`.
export const deleteNote = (id: string) => invoke<void>('delete', { id });
export const search = (query: string) => invoke<ObjectMeta[]>('search', { query });
export const recent = () => invoke<ObjectMeta[]>('recent');

// Asset bytes for the webview. `kind` picks the derived thumbnail or the full
// blob; the caller wraps the ArrayBuffer in an object URL. Returns null in the
// browser/test mock (no vault), so callers fall back to the missing placeholder.
export const resolveAsset = (reference: string, kind: 'full' | 'thumb') =>
  invoke<ArrayBuffer | null>('resolve_asset', { reference, kind });
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
export const ping = () => invoke<{ changed: boolean; git: boolean }>('ping');

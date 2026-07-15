import type { AssetStatus, Board, NoteDetail, ObjectMeta } from './types';
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
export const getGallery = () => invoke<ObjectMeta[]>('gallery');
export const getAgenda = () => invoke<ObjectMeta[]>('agenda');
export const getNote = (id: string) => invoke<NoteDetail | null>('get', { id });
export const capture = (body: string) => invoke<ObjectMeta>('capture', { body });
export const setProperty = (id: string, key: string, value: string) =>
  invoke<void>('set_property', { id, key, value });
export const updateBody = (id: string, body: string) =>
  invoke<void>('update_body', { id, body });
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
// insert a reference. Raw bytes go in the POST body; the name rides the query.
export async function ingestFile(file: File): Promise<ObjectMeta> {
  if (import.meta.env.PROD) {
    const res = await fetch(`/api/ingest?name=${encodeURIComponent(file.name)}`, {
      method: 'POST',
      body: file,
    });
    if (!res.ok) throw new Error((await res.text()) || res.statusText);
    return res.json();
  }
  return mock.handle<ObjectMeta>('ingest', { name: file.name });
}

// Durability: commit the vault's notes to its own git repo (returns true if a
// commit was made), and snapshot it to restic (repo + password from the env).
export const commit = (message: string) => invoke<boolean>('commit', { message });
export const backup = () => invoke<void>('backup');

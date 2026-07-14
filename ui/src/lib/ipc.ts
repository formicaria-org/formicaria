import type { Board, ObjectMeta } from './types';
import * as mock from './mock';

// Inside the Tauri window we call the Rust commands over IPC. In a plain browser
// (`pnpm dev` with no shell) we fall back to an in-memory mock, so the board
// stays developable without launching the desktop app. Tauri v2 marks its
// windows with __TAURI_INTERNALS__.
const inTauri = typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== 'undefined';

async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  return mock.handle<T>(cmd, args);
}

// Argument keys are camelCase; Tauri v2 maps them to the Rust snake_case params
// (groupBy -> group_by). The single-word ones pass through unchanged.
export const getBoard = (groupBy: string) => invoke<Board>('board', { groupBy });
export const getGallery = () => invoke<ObjectMeta[]>('gallery');
export const getAgenda = () => invoke<ObjectMeta[]>('agenda');
export const capture = (body: string) => invoke<ObjectMeta>('capture', { body });
export const setProperty = (id: string, key: string, value: string) =>
  invoke<void>('set_property', { id, key, value });

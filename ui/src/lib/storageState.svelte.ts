// **Unused attachments, what Settings is showing** (2026-11-21): kept here rather than in the panel,
// which is drawn again when the notes change — and putting a file aside deletes its attachment entry,
// so the list and what was said vanished the moment they mattered (found in formicaria's browser).
import type { StorageReport } from './ipc';

export const storage = $state<{
  unused: Record<string, StorageReport>;
  picked: Record<string, string[]>;
  said: Record<string, string>;
  busy: boolean;
}>({ unused: {}, picked: {}, said: {}, busy: false });

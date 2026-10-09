// The app's undo stack against the mock backend — `decisions.md` 2026-10-09.
import { afterEach, beforeEach, expect, test } from 'vitest';
import { capture, getNote, setProperty, updateBody } from './ipc';
import { reset } from './mock';
import * as undo from './undo.svelte';

beforeEach(() => undo.clear());
afterEach(() => reset());

async function fresh(body = 'hello') {
  return capture(body, '');
}

test('a property change is undone and redone', async () => {
  const n = await fresh();
  await undo.setPropertyUndoable(n.id, 'status', 'doing', 'changed the status');
  expect(undo.nextUndo()?.label).toBe('changed the status');
  await undo.undo();
  expect((await getNote(n.id))?.status ?? null).toBeNull();
  await undo.redo();
  expect((await getNote(n.id))?.status).toBe('doing');
});

test('an undo refuses to overwrite a change made since, and steps aside', async () => {
  const n = await fresh();
  await undo.setPropertyUndoable(n.id, 'due', '2026-10-16', 'first');
  await undo.setPropertyUndoable(n.id, 'status', 'doing', 'second');
  // Someone else moves the due date after this session's change.
  await setProperty(n.id, 'status', 'done');
  await expect(undo.undo()).rejects.toThrow(/changed again since/);
  expect((await getNote(n.id))?.status).toBe('done');
  // The refused step is dropped, so the one before it is next — not stuck forever.
  expect(undo.nextUndo()?.label).toBe('first');
  await undo.undo();
  expect((await getNote(n.id))?.due ?? null).toBeNull();
});

test('a deleted note comes back exactly, and only once', async () => {
  const n = await fresh('the plots are in folder B');
  await undo.deleteUndoable(n.id, 'Plots');
  expect(await getNote(n.id)).toBeNull();
  await undo.undo();
  expect((await getNote(n.id))?.body).toBe('the plots are in folder B');
  await undo.redo();
  expect(await getNote(n.id)).toBeNull();
});

test('a new action clears redo', async () => {
  const n = await fresh();
  await undo.setPropertyUndoable(n.id, 'status', 'doing', 'a');
  await undo.undo();
  expect(undo.nextRedo()?.label).toBe('a');
  await undo.setPropertyUndoable(n.id, 'status', 'done', 'b');
  expect(undo.nextRedo()).toBeNull();
});

test('a finished editing session is one step, guarded by the text it left', async () => {
  const n = await fresh('before');
  const d = await getNote(n.id);
  await updateBody(n.id, 'after', d!.version);
  undo.recordTextEdit(n.id, 'Note', 'before', 'after');
  await undo.undo();
  expect((await getNote(n.id))?.body).toBe('before');
});

test('setting a value to what it already was records nothing', async () => {
  const n = await fresh();
  await undo.setPropertyUndoable(n.id, 'status', '', 'noop');
  expect(undo.nextUndo()).toBeNull();
});

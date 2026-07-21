// The discussions mock, driven as the Discussions view and "New discussion" drive it. A discussion
// is a self-rooted note (`thread_of` points at itself); the mock's `isDiscussion` must agree with
// the server's `is_discussion_root`, or dev and CI would see a different world from production.

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { DiscussionSummary, ObjectMeta, ThreadView } from './types';

const newDiscussion = (title: string) =>
  mock.handle<ObjectMeta>('create_discussion', { title, vault: '' });
const reply = (id: string, body: string) => mock.handle<ObjectMeta>('reply', { id, body });
const discussions = () => mock.handle<DiscussionSummary[]>('discussions', {});
const thread = (id: string) => mock.handle<ThreadView>('thread', { id });
const recent = () => mock.handle<ObjectMeta[]>('recent', {});

describe('discussions in the mock backend', () => {
  it('creates a self-rooted discussion, listed but kept out of the note views', async () => {
    const beforeRecent = (await recent()).length;
    const beforeCount = (await discussions()).length;

    const d = await newDiscussion('Drop the second arm?');
    expect(d.props.thread_of).toBe(`note:${d.id}`); // the self-anchor

    const list = await discussions();
    expect(list.length).toBe(beforeCount + 1);
    expect(list.some((x) => x.id === d.id)).toBe(true);
    // A discussion is not a note you plan — the note list/`/`-picker is unchanged.
    expect((await recent()).length).toBe(beforeRecent);
  });

  it('a reply joins the discussion, and the root is not a message in its own thread', async () => {
    const d = await newDiscussion('Weekly sync');
    const m = await reply(d.id, 'kicking it off');
    expect(m.props.thread_of).toBe(`note:${d.id}`);

    const t = await thread(d.id);
    expect(t.count).toBe(1);
    expect(t.messages.every((msg) => msg.id !== d.id)).toBe(true);
  });

  it('refuses a titleless discussion', async () => {
    await expect(newDiscussion('   ')).rejects.toThrow();
  });
});

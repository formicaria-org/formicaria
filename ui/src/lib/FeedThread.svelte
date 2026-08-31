<script lang="ts">
  /// **A note's discussion, inside a feed post.** The last few messages and one box to add another.
  ///
  /// **Deliberately not `NotePanel`'s discussion block.** That one is the full surface — the
  /// `@`-mention picker, agent command chips, the live agent-activity poll, `proposal_for`, a
  /// per-message reply target and a two-click delete. Reusing it here would drag every one of those
  /// into a scrolling list: `online_agents` + `agent_activity` every four seconds *per open post*,
  /// a `discussions()` full-corpus scan to build the mention list, and a `svelte:window` keydown
  /// handler per instance. All of that still works, one tap away, in the note. See `decisions.md`,
  /// 2026-08-31.
  ///
  /// **Flat, not threaded.** `reply` re-roots automatically — replying to a message joins the same
  /// discussion rather than starting an unreachable one — so "flat" is literally *not passing a
  /// message id*, which is cheaper as well as narrower. Four levels of `depth * 1.1rem` indent in a
  /// 340 px phone card leaves a 40 px column, which is not a conversation anyone can read.
  import { thread as ipcThread, reply as ipcReply } from './ipc';
  import type { ThreadMessage } from './types';
  import EditedBy from './EditedBy.svelte';
  import { lastEditFor } from './activity.svelte';

  interface Props {
    noteId: string;
    /// Called after a message lands, with the new count — the feed keeps the badge, so it does not
    /// have to re-ask. **Never a `refresh()`**: that re-runs `recent()` *and* `activity()`'s git
    /// revwalk under the vault lock, which is the most expensive possible answer to a 200-byte
    /// write.
    onposted?: (count: number) => void;
    /// The editor's own commit signal. A message is a file write like any other, so it rides the
    /// same debounce rather than opening a second path to git.
    onsaved?: () => void;
    /// Read the whole conversation — the feed shows the tail.
    onopen?: (id: string) => void;
  }
  let { noteId, onposted, onsaved, onopen }: Props = $props();

  /// How many messages a post shows before deferring to the note. The rest is one tap away, and a
  /// feed row is not where a forty-message thread should be read.
  const TAIL = 3;

  let messages = $state<ThreadMessage[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let draft = $state('');
  let busy = $state(false);
  let error = $state('');
  let box = $state<HTMLTextAreaElement | undefined>(undefined);

  /// **Keyed on the note id, never on the note object.** `NotePanel` carries a memo barrier for
  /// exactly this reason: an effect that re-ran whenever the note *object* changed fired on every
  /// autosave, which cost a full-corpus `backlinks` read per keystroke and silently closed the
  /// open thread. A feed row's identity and its content are the same object, so this is the first
  /// place that bug would come back.
  $effect(() => {
    const id = noteId;
    let cancelled = false;
    loading = true;
    void ipcThread(id)
      .then((v) => {
        if (cancelled) return;
        messages = v.messages;
        total = v.count;
        loading = false;
      })
      .catch((e) => {
        if (cancelled) return;
        error = e instanceof Error ? e.message : String(e);
        loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  const tail = $derived(messages.slice(-TAIL));

  async function send() {
    const body = draft.trim();
    if (!body || busy) return;
    busy = true;
    error = '';
    try {
      await ipcReply(noteId, body);
      // Append locally rather than re-reading: one write should not cost a whole-corpus re-read,
      // and re-fetching the feed here would collapse the reader's window on the next beat.
      const v = await ipcThread(noteId);
      messages = v.messages;
      total = v.count;
      draft = '';
      onposted?.(v.count);
      onsaved?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  /// The Android keyboard covers the space below the caret — `NotePanel` records this. A composer
  /// halfway down a scrolling feed is exactly where that bites.
  function onFocus() {
    box?.scrollIntoView({ block: 'nearest' });
  }
</script>

<section class="ft" aria-label="Discussion">
  {#if loading}
    <p class="muted">Reading the discussion…</p>
  {:else}
    {#if total > tail.length}
      <button class="more" onclick={() => onopen?.(noteId)}>
        {total - tail.length} earlier {total - tail.length === 1 ? 'message' : 'messages'} — open the note
      </button>
    {/if}
    {#each tail as m (m.id)}
      <article class="msg">
        <EditedBy edit={lastEditFor(m.id)} />
        <!-- Plain text, exactly as the note panel shows a message. Rendering Markdown in a list is
             refused: it re-enters the full-blob image path at N per screen. -->
        <p class="body">{m.body}</p>
      </article>
    {/each}
    {#if !total}
      <p class="muted">No messages yet.</p>
    {/if}
  {/if}

  <div class="composer">
    <textarea
      bind:this={box}
      bind:value={draft}
      onfocus={onFocus}
      rows="1"
      placeholder="Write a message…"
      aria-label="write a message"
      disabled={busy}></textarea>
    <button class="send" onclick={send} disabled={busy || !draft.trim()}>Send</button>
  </div>
  {#if error}<p class="err">{error}</p>{/if}
</section>

<style>
  .ft {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  .more {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    text-decoration: underline;
    cursor: pointer;
  }
  .msg {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .msg .body {
    margin: 0;
    font-size: var(--text-sm);
    white-space: pre-wrap;
  }
  .composer {
    display: flex;
    gap: var(--space-2);
    align-items: flex-end;
  }
  .composer textarea {
    flex: 1 1 auto;
    min-width: 0;
    min-height: 2.25rem;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    resize: vertical;
  }
  .send {
    flex: 0 0 auto;
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .send:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .err {
    margin: 0;
    color: var(--danger-fg, var(--text));
    font-size: var(--text-xs);
  }
  /* A thumb uses this on a phone. */
  @media (pointer: coarse) {
    .composer textarea,
    .send {
      min-height: 2.75rem;
    }
  }
</style>

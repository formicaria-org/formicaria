<script lang="ts">
  import StatusChip from '../lib/StatusChip.svelte';
  import VaultBadge from '../lib/VaultBadge.svelte';
  import EditedBy from '../lib/EditedBy.svelte';
  import { lastEditFor } from '../lib/activity.svelte';
  import { dayHeading, ymd } from '../lib/calendar';
  import type { ObjectMeta } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';
  import AssetMissing from '../lib/AssetMissing.svelte';
  import Icon from '../lib/Icon.svelte';
  import { assetUrl } from '../lib/ipc';
  import { relativeTime } from '../lib/stamp';

  // A Logseq-style journal: every note under the day it was created, newest day
  // first. Cards arrive already sorted newest-created-first (the `recent` query),
  // so same-day notes are contiguous and we just partition them into day buckets
  // in order. No status literal here — renderer-safe.
  let {
    cards,
    onopen,
    statuses = [],
    onstatus,
    mode = 'feed',
    counts = {},
    expandedId = null,
    ontoggle,
    thread,
  }: {
    cards: ObjectMeta[];
    onopen: (id: string) => void;
    statuses?: string[];
    onstatus?: (id: string, value: string | null) => void;
    /// `feed` shows each note as a post — its picture, its excerpt, who touched it last.
    /// `compact` is the original one-line row. A pane preference, like the agenda's month/week/list.
    mode?: 'feed' | 'compact';
    /// How many messages each note's discussion holds, by note id — from **one** `thread_roots`
    /// call for the whole feed. Absent means none; a post with no conversation costs nothing.
    counts?: Record<string, number>;
    /// **One thread open at a time.** Not a per-row flag: reading a discussion is a whole-corpus
    /// query (`fm-app/tests/perf.rs`), and N open threads would also be N polls. One at a time
    /// keeps the marginal cost of the whole feature at one of each.
    expandedId?: string | null;
    ontoggle?: (id: string) => void;
    /// What to draw inside the expanded post. A snippet rather than an import, so this renderer
    /// stays a renderer — it knows nothing about how a discussion is fetched or posted.
    thread?: import('svelte').Snippet<[string]>;
  } = $props();

  /// **A window, because there is no virtualisation anywhere in this app and `recent` is
  /// unbounded** — it returns every note in every vault in scope, and a row per note was already
  /// the ceiling. A post carries an image, so the same list now costs decoded pixels as well as
  /// DOM. Thirty is a number picked to be changed once someone has watched it on a real vault;
  /// what matters is that the cap is visible and has a button, rather than silently truncating.
  const PAGE = 30;
  let shown = $state(PAGE);
  /// **Reset when the underlying set changes — not when it merely moves.** This read
  /// `void cards.length`, which `refresh()` re-triggers on every `changed` poll beat because it
  /// replaces the whole array wholesale. So any vault change collapsed the window, and once you
  /// can post a reply from the feed, *your own reply* would collapse your window and scroll
  /// position on the next beat. Keying on the first row's identity keeps the intent (switching
  /// vault or filter lands you at the top) while an edit, a new note or a reply leaves the window
  /// where the reader put it.
  let anchor = $derived(cards[0]?.id ?? '');
  $effect(() => {
    void anchor;
    shown = PAGE;
  });

  /// The picture for a post: the note's first asset, as a **thumbnail**. Nothing in the app asked
  /// for a thumbnail before this — `render.ts` records that every inline image decoded the full
  /// blob, "~50 MB of decoded pixels" for one 12 MP photo. In a feed that would be per screen.
  /// `assets` already rides on every note (`dto.rs`: "so no extra fetch is required to render a
  /// preview"), so this costs no request.
  const pictureOf = (c: ObjectMeta) => (c.assets && c.assets.length ? c.assets[0] : null);
  let broken = $state(new Set<string>());

  /// The window applies to the feed only. The compact list keeps the behaviour it always had, so
  /// this change cannot alter the view that already existed.
  let visible = $derived(mode === 'feed' ? cards.slice(0, shown) : cards);

  let days = $derived.by(() => {
    const out: { key: string; heading: string; items: ObjectMeta[] }[] = [];
    for (const c of visible) {
      // **The note's *local* day, not the UTC one.** `created` is an ISO instant, so slicing
      // its first ten characters gives the day in UTC — while `dayHeading` compares against the
      // local day. East of Greenwich those disagree from midnight until the offset elapses: at
      // UTC+8 a note written at 00:30 was filed under "Yesterday" every night until 08:00.
      // Caught by a test that ran past midnight.
      const key = ymd(new Date(c.created));
      let bucket = out[out.length - 1];
      if (!bucket || bucket.key !== key) {
        bucket = { key, heading: dayHeading(key), items: [] };
        out.push(bucket);
      }
      bucket.items.push(c);
    }
    return out;
  });
</script>

<div class="timeline">
  {#if cards.length === 0}
    <EmptyState
      icon="timeline"
      title="No notes yet"
      hint="Press + to write your first one. Everything you write shows up here, newest day first."
    />
  {:else}
    <div class="feed">
      {#each days as day (day.key)}
        <section class="day">
          <h2 class="heading">
            <span>{day.heading}</span>
            <span class="count">{day.items.length}</span>
          </h2>
          <div class="items">
            {#each day.items as card (card.id)}
              <!-- A div, not a button: the status chip nested here is itself a
                   button, and a button inside a button is invalid HTML. Same
                   role/tabindex pattern the board's Card already uses. -->
              {#if mode === 'feed'}
                <!-- **A post is an article, not a button.** It used to be a `div` with
                     `role="button"`, which makes it a *leaf* in the accessibility tree: everything
                     inside flattens into one accessible name and the `StatusChip` within it is not
                     separately reachable. That was survivable while a post held only a chip; it is
                     not once it holds a thread and a composer, which would be a form inside a
                     button. So the post is plain content and the interactive parts are real
                     controls. See `decisions.md`, 2026-08-31. -->
                <article class="row post" data-type={card.type}>
                  <!-- **Who, and how long ago — at the top, like any feed.** This is the half that
                       makes a shared vault legible: one stream across every vault you are an
                       audience for, each post saying which one it came from and who last touched
                       it. -->
                  <header class="byline">
                    <EditedBy edit={lastEditFor(card.id)} />
                    <VaultBadge vault={card.vault} />
                    <span class="when">{relativeTime(card.created)}</span>
                  </header>

                  <!-- **The one control that opens the note**, and it wraps the picture as well as
                       the title so most of the post is still a target — which is what a card
                       clicked anywhere used to buy, recovered without an ambiguous handler. There
                       is deliberately no double-tap: it would collide with the chip's tap and
                       long-press, contradict "double-click means edit", and have no keyboard
                       form. -->
                  <button class="open" onclick={() => onopen(card.id)}>
                    {#if pictureOf(card)}
                      {@const ref = pictureOf(card)!}
                      <span class="shot">
                        {#if broken.has(ref)}
                          <!-- A note whose media has not synced yet is the ordinary case in a
                               shared vault, so it must read as "not here yet", never as broken. -->
                          <AssetMissing />
                        {:else}
                          <img
                            src={assetUrl(ref, 'thumb')}
                            alt={card.title ? `Picture in “${card.title}”` : 'Picture in this note'}
                            loading="lazy"
                            decoding="async"
                            onerror={() => (broken = new Set(broken).add(ref))}
                          />
                        {/if}
                      </span>
                    {/if}
                    <span class="title">{card.title ?? card.preview ?? 'Untitled'}</span>
                  </button>

                  <!-- `excerpt` is the feed's own field (several lines, char-capped server-side);
                       `preview` is the one line every other list gets. Falling back keeps this
                       honest against a payload that predates the field, and against the compact
                       list below, which is deliberately never sent one. -->
                  {#if card.title && (card.excerpt ?? card.preview)}
                    <p class="preview">{card.excerpt ?? card.preview}</p>
                  {/if}

                  <footer class="tags">
                    {#if onstatus}
                      <StatusChip
                        status={card.status}
                        {statuses}
                        onchange={(next) => onstatus?.(card.id, next)}
                      />
                    {/if}
                    {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
                    {#if ontoggle}
                      <!-- The count comes from one `thread_roots` call for the whole feed, never a
                           `thread()` per row — that one is a whole-corpus read (see
                           `fm-app/tests/perf.rs`), so thirty of them would be thirty corpus scans
                           behind one mutex. Most of the value is simply knowing a conversation
                           exists; opening one is the expensive half and happens on demand. -->
                      <button
                        class="comments"
                        aria-expanded={expandedId === card.id}
                        onclick={() => ontoggle?.(card.id)}
                        title={counts[card.id] ? 'Show the discussion' : 'Start a discussion'}
                      >
                        <Icon name="chat" size={14} />
                        {#if counts[card.id]}<span class="n">{counts[card.id]}</span>{/if}
                      </button>
                    {/if}
                  </footer>

                  {#if expandedId === card.id && thread}
                    {@render thread(card.id)}
                  {/if}
                </article>
              {:else}
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <div
                  class="row"
                  data-type={card.type}
                  role="button"
                  tabindex="0"
                  onclick={() => onopen(card.id)}
                  onkeydown={(e) =>
                    (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onopen(card.id))}
                >
                  {#if onstatus}
                    <StatusChip
                      status={card.status}
                      {statuses}
                      onchange={(next) => onstatus?.(card.id, next)}
                    />
                  {:else}
                    <span class="type" data-type={card.type}>{card.type}</span>
                  {/if}
                  <span class="what">
                    <span class="title">{card.title ?? card.preview ?? card.id}</span>
                    {#if card.title && card.preview}<span class="preview">{card.preview}</span>{/if}
                    {#if card.vault || card.tags.length}
                      <span class="tags">
                        <VaultBadge vault={card.vault} />
                        <EditedBy edit={lastEditFor(card.id)} />
                        {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
                      </span>
                    {/if}
                  </span>
                </div>
              {/if}
            {/each}
          </div>
        </section>
      {/each}
      {#if mode === 'feed' && cards.length > shown}
        <!-- The cap says so and offers the way past. Silently truncating would be the same bug as
             a board that does not admit it scrolls sideways — the reader cannot tell a short list
             from a short vault. -->
        <button class="more" onclick={() => (shown += PAGE)}>
          Show more — {cards.length - shown} older
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .timeline {
    height: 100%;
    overflow-y: auto;
    padding: 1rem;
    box-sizing: border-box;
  }
  .feed {
    margin: 0 auto;
    max-width: 46rem;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }
  /* **Not sticky.** It was pinned to the top of the scroller, which is fine over one-line rows and
     wrong over a feed of tall cards: the pinned date is an opaque band that slides across whatever
     post is passing under it, so the note you are actually reading is the one it covers. Reported
     2026-08-31 — *"the date of the previous blocks appear over text on top notes, making it
     unreadable"*. The date now scrolls away with the day it belongs to, which is the only moment
     it can never intersect anything. Every post also carries its own relative time in the byline,
     so nothing is lost while the heading is off screen. */
  .heading {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin: 0 0 0.4rem;
    padding: 0.2rem 0;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--accent);
    border-bottom: 1px solid var(--column-border);
  }
  .count {
    font-size: 0.7rem;
    font-weight: 400;
    color: var(--muted);
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  /* ---- A post ------------------------------------------------------------------------------
     Stacked rather than the row's left-to-right: byline, picture, words, tags. The picture is the
     widest thing in it, which is what makes this read as a feed rather than a taller list. */
  /* **`.row.post`, not `.post`.** A post carries both classes, and `.row` is declared further
     down this file — equal specificity, so source order decided and `.row` silently overrode
     `display`, `padding`, `gap`, `background` and `border`. The stacked post described above was
     not what rendered. Same class of defect `ci/checks.sh` already polices for `.panel-views`. */
  .row.post {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: stretch;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
  }
  /* The one control that opens the note. Full width and holding the picture as well as the title,
     so most of the post is still a target — what clicking anywhere used to buy, without the
     ambiguous handler. */
  .post .open {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .post .open:hover .title {
    text-decoration: underline;
  }
  .post footer.tags {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .post .comments {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .post .comments:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .post .comments[aria-expanded='true'] {
    border-color: var(--accent);
    color: var(--text);
  }
  .byline {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .byline .when {
    margin-left: auto;
    color: var(--text-subtle);
    font-size: var(--text-xs);
  }
  .shot {
    /* Bounded by height, not width: a portrait photo must not push the words off the screen, and
       a panorama must not become a hairline. `cover` on a fixed band is what every feed does. */
    max-height: 22rem;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--surface-hover);
    display: flex;
  }
  .shot img {
    width: 100%;
    max-height: 22rem;
    object-fit: cover;
    display: block;
  }
  .post .preview {
    margin: 0;
  }
  .post .title {
    font-size: var(--text-md);
    line-height: var(--lh-sm);
    font-weight: 600;
  }
  .post .preview {
    color: var(--text-muted);
    /* **Several lines, fading out.** The server sends a char-capped excerpt rather than a fraction
       of the body (the cap is what keeps `recent()` bounded — see `dto.rs`), so the clamp decides
       how much of it a post spends, and the mask says "there is more" without a control that
       claims to know how much more. Six lines is a post you can read; the note is one tap away.

       `white-space: pre-line` because the excerpt keeps its line breaks — a heading and its first
       paragraph read as two lines, which is the shape people recognise. */
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: pre-line;
    mask-image: linear-gradient(to bottom, #000 70%, transparent 100%);
  }
  /* The compact list is a line per note and is never sent an excerpt; keep its old clamp. */
  .row:not(.post) .preview {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    align-self: center;
    margin: var(--space-3) 0 var(--space-5);
    padding: var(--space-2) var(--space-4);
    min-height: 2.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .more:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .row {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    gap: 0.7rem;
    padding: 0.55rem 0.8rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
    width: 100%;
    transition:
      border-color var(--dur-fast) var(--ease),
      box-shadow var(--dur-fast) var(--ease);
  }
  .row:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-sm);
  }
  .type {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--card-border);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    margin-top: 0.1rem;
  }
  .what {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .title {
    color: var(--text);
    font-size: 0.9rem;
    overflow-wrap: anywhere;
  }
  .preview {
    font-size: 0.76rem;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
    margin-top: 0.1rem;
  }
  .tag {
    font-size: 0.68rem;
    padding: 0.02rem 0.35rem;
    border-radius: 999px;
    background: var(--tag-bg);
    color: var(--tag-fg);
  }
</style>

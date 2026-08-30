<script lang="ts">
  import StatusChip from '../lib/StatusChip.svelte';
  import VaultBadge from '../lib/VaultBadge.svelte';
  import EditedBy from '../lib/EditedBy.svelte';
  import { lastEditFor } from '../lib/activity.svelte';
  import { dayHeading, ymd } from '../lib/calendar';
  import type { ObjectMeta } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';
  import AssetMissing from '../lib/AssetMissing.svelte';
  import { assetUrl } from '../lib/ipc';
  import { relativeTime } from '../lib/stamp';

  // A Logseq-style journal: every note under the day it was created, newest day
  // first. Cards arrive already sorted newest-created-first (the `recent` query),
  // so same-day notes are contiguous and we just partition them into day buckets
  // in order. No status literal here — renderer-safe.
  let { cards, onopen, statuses = [], onstatus, mode = 'feed' }: {
    cards: ObjectMeta[];
    onopen: (id: string) => void;
    statuses?: string[];
    onstatus?: (id: string, value: string | null) => void;
    /// `feed` shows each note as a post — its picture, its first line, who touched it last.
    /// `compact` is the original one-line row. A pane preference, like the agenda's month/week/list.
    mode?: 'feed' | 'compact';
  } = $props();

  /// **A window, because there is no virtualisation anywhere in this app and `recent` is
  /// unbounded** — it returns every note in every vault in scope, and a row per note was already
  /// the ceiling. A post carries an image, so the same list now costs decoded pixels as well as
  /// DOM. Thirty is a number picked to be changed once someone has watched it on a real vault;
  /// what matters is that the cap is visible and has a button, rather than silently truncating.
  const PAGE = 30;
  let shown = $state(PAGE);
  /// Reset when the underlying set changes — switching vault or filter otherwise leaves the window
  /// opened onto a list that is no longer there.
  $effect(() => {
    void cards.length;
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
      hint="Press + to write your first one. Everything you write shows up here, newest day first." />
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
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div
                class="row"
                class:post={mode === 'feed'}
                data-type={card.type}
                role="button"
                tabindex="0"
                onclick={() => onopen(card.id)}
                onkeydown={(e) =>
                  (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onopen(card.id))}
              >
                {#if mode === 'feed'}
                  <!-- **Who, and how long ago — at the top, like any feed.** This is the half that
                       makes a shared vault legible: one stream across every vault you are an
                       audience for, each post saying which one it came from and who last touched
                       it. Both components already existed and are already used on the row below;
                       here they lead rather than trail. -->
                  <header class="byline">
                    <EditedBy edit={lastEditFor(card.id)} />
                    <VaultBadge vault={card.vault} />
                    <span class="when">{relativeTime(card.created)}</span>
                  </header>

                  {#if pictureOf(card)}
                    {@const ref = pictureOf(card)!}
                    <div class="shot">
                      {#if broken.has(ref)}
                        <!-- The placeholder that was built for exactly this and never called from
                             anywhere: its own comment names "gallery tile, card chip" as its homes.
                             A note whose media has not synced yet is the ordinary case in a shared
                             vault, so it must read as "not here yet", never as a broken app. -->
                        <AssetMissing />
                      {:else}
                        <img
                          src={assetUrl(ref, 'thumb')}
                          alt={card.title ? `Picture in “${card.title}”` : 'Picture in this note'}
                          loading="lazy"
                          decoding="async"
                          onerror={() => (broken = new Set(broken).add(ref))} />
                      {/if}
                    </div>
                  {/if}

                  <span class="what">
                    <span class="title">{card.title ?? card.preview ?? 'Untitled'}</span>
                    {#if card.title && card.preview}<span class="preview">{card.preview}</span>{/if}
                    <span class="tags">
                      {#if onstatus}
                        <StatusChip
                          status={card.status}
                          {statuses}
                          onchange={(next) => onstatus?.(card.id, next)}
                        />
                      {/if}
                      {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
                    </span>
                  </span>
                {:else}
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
                {/if}
              </div>
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
  .heading {
    position: sticky;
    top: 0;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin: 0 0 0.4rem;
    padding: 0.2rem 0;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--accent);
    background: var(--bg);
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
  .post {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: stretch;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
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
  .post .what {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }
  .post .title {
    font-size: var(--text-md);
    line-height: var(--lh-sm);
    font-weight: 600;
  }
  .post .preview {
    color: var(--text-muted);
    /* Two lines of caption. The first line of the note is all that is sent, and clamping keeps a
       long one from setting the height of every post below it. */
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

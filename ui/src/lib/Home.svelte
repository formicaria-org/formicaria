<script lang="ts">
  /// **Home: what this is and where to go, in four pictures** (`decisions.md` 2026-10-18, *a Home
  /// a newcomer starts on and can always come back to*). A person who does not program installed
  /// the app, chose Agenda from the first list they saw, and then asked *"I want to go back to the
  /// home page"* — they did not know they could start writing, or move between views. The words
  /// that answered it were in a 400-word note on the computer and nowhere on the phone.
  ///
  /// So: built into the app (never a note it writes into someone's notebook), first in the list of
  /// views on every device, and short — a picture and a line per idea, and buttons that do the
  /// thing rather than describe it. The manual is one press away for anything longer.
  import Icon from './Icon.svelte';
  import { openManual } from './manual.svelte';
  import type { PaneKind } from './panes';

  let { ongo }: { /** Show another view in this place. */ ongo: (kind: PaneKind) => void } =
    $props();

  const VIEWS: { kind: PaneKind; label: string; icon: string; says: string }[] = [
    { kind: 'timeline', label: 'Timeline', icon: 'timeline', says: 'everything, newest first' },
    { kind: 'board', label: 'Board', icon: 'board', says: 'cards in columns' },
    { kind: 'agenda', label: 'Agenda', icon: 'calendar', says: 'anything with a date' },
  ];
</script>

<div class="home">
  <h2>Your notebook</h2>
  <p class="lede">Everything stays on this device. Here is all you need to start.</p>

  <ol class="ideas">
    <li>
      <span class="pic"><Icon name="plus" size={22} /></span>
      <div>
        <h3>Write something</h3>
        <p>Press <strong>＋</strong>, then <strong>New note</strong>. Just start typing.</p>
      </div>
    </li>
    <li>
      <span class="pic"><Icon name="views" size={22} /></span>
      <div>
        <h3>See your notes your way</h3>
        <p>
          The same notes, shown differently. Switch any time here, or with
          <span class="inline" aria-label="the views button"><Icon name="views" size={15} /></span>:
        </p>
        <div class="go">
          {#each VIEWS as v (v.kind)}
            <button type="button" onclick={() => ongo(v.kind)}>
              <Icon name={v.icon} size={18} />
              <span><strong>{v.label}</strong> <small>{v.says}</small></span>
            </button>
          {/each}
        </div>
      </div>
    </li>
    <li>
      <span class="pic"><Icon name="backup" size={22} /></span>
      <div>
        <h3>Keep them safe</h3>
        <p><strong>Back up</strong> keeps a copy somewhere else you choose.</p>
      </div>
    </li>
    <li>
      <span class="pic"><Icon name="home" size={22} /></span>
      <div>
        <h3>Come back here</h3>
        <p>
          Press <span class="inline" aria-label="the views button"
            ><Icon name="views" size={15} /></span
          >
          and choose <strong>Home</strong> — always the first one.
        </p>
      </div>
    </li>
  </ol>

  <p class="more">
    <button type="button" class="link" onclick={() => openManual('user/first-note.html')}
      >Show me more: the manual</button
    >
  </p>
</div>

<style>
  .home {
    box-sizing: border-box;
    max-width: 34rem;
    margin: 0 auto;
    padding: var(--space-5) var(--space-4) var(--space-6);
    overflow-y: auto;
    height: 100%;
  }
  h2 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-xl);
    line-height: var(--lh-xl);
  }
  .lede {
    margin: 0 0 var(--space-5);
    color: var(--text-muted);
  }
  .ideas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .ideas > li {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
  }
  .pic {
    flex: none;
    display: grid;
    place-items: center;
    width: 2.75rem;
    height: 2.75rem;
    border-radius: var(--radius-pill);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  h3 {
    margin: 0.35rem 0 var(--space-1);
    font-size: var(--text-md);
  }
  p {
    margin: 0;
    color: var(--text-muted);
    line-height: var(--lh-base);
  }
  /* The very picture on the button the line talks about, so the two are recognised as one. */
  .inline {
    display: inline-grid;
    place-items: center;
    vertical-align: -0.2em;
    width: 1.5em;
    height: 1.5em;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
  }
  .go {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .go button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .go button:hover {
    background: var(--surface-hover);
  }
  .go small {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .more {
    margin-top: var(--space-6);
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--link);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
</style>

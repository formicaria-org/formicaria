<script lang="ts">
  // **The player bar** (`decisions.md#ui`, 2026-11-13, *a sixth group, Music*, M3): the app's own,
  // under the windows on a computer and above the bar on a phone. It is drawn **only once something
  // is in the queue**, so nothing is laid over the views until a song is played.
  //
  // Two layouts of one bar, decided by width as the rest of the shell is: a wide screen shows the
  // position and the volume in the bar; a narrow one shows the song and play/pause, and its press
  // opens **Now playing** — the position, the controls and the queue, one thing on the screen.
  import Icon from '../Icon.svelte';
  import { assetUrl } from '../ipc';
  import * as nav from '../nav.svelte';
  import * as player from './player.svelte';

  interface Props {
    /** Open the song's own note in the workspace. */
    onopen?: (id: string) => void;
  }
  let { onopen }: Props = $props();

  const t = $derived(player.now());
  let open = $state(false);
  $effect(() => {
    if (open && t && !player.deckShown()) nav.openLayer('now-playing', () => (open = false));
    else nav.closeLayer('now-playing');
  });
  $effect(() => () => nav.closeLayer('now-playing'));

  const clock = (secs: number) => {
    const s = Math.max(0, Math.floor(secs));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  };
  const by = (x: player.Track) => [x.artist, x.album].filter(Boolean).join(' · ');
</script>

{#snippet controls(big: boolean)}
  <button
    type="button"
    class="ctl"
    onclick={player.previous}
    aria-label="previous song"
    title="Previous"
  >
    <Icon name="skip-back" size={big ? 26 : 18} />
  </button>
  <button
    type="button"
    class="ctl main"
    class:big
    onclick={player.toggle}
    aria-label={player.playing() ? 'pause' : 'play'}
    title={player.playing() ? 'Pause' : 'Play'}
  >
    <Icon name={player.playing() ? 'pause' : 'play'} size={big ? 30 : 20} />
  </button>
  <button
    type="button"
    class="ctl"
    onclick={player.next}
    disabled={!player.hasNext()}
    aria-label="next song"
    title="Next"
  >
    <Icon name="skip-forward" size={big ? 26 : 18} />
  </button>
{/snippet}

{#snippet position()}
  <label class="pos">
    <span class="time">{clock(player.time())}</span>
    <input
      type="range"
      min="0"
      max={Math.max(1, Math.round(player.length()))}
      step="1"
      value={Math.round(player.time())}
      oninput={(e) => player.seek(Number((e.currentTarget as HTMLInputElement).value))}
      aria-label="position in the song"
    />
    <span class="time">{player.length() ? clock(player.length()) : '–:––'}</span>
  </label>
{/snippet}

{#if t && !player.deckShown()}
  <div class="player" role="region" aria-label="player">
    <button
      type="button"
      class="song"
      onclick={() => {
        // In the Music view, its deck is the place for the song; elsewhere, Now playing.
        if (!player.toDeck()) open = true;
      }}
      aria-label={`now playing: ${t.title}`}
      aria-expanded={open}
    >
      {#if t.cover}
        <img class="art" src={assetUrl(t.cover, 'thumb')} alt="" />
      {:else}
        <span class="art none" aria-hidden="true"></span>
      {/if}
      <span class="name">
        <span class="title">{t.title}</span>
        {#if by(t)}<span class="by">{by(t)}</span>{/if}
      </span>
    </button>
    <div class="ctls">{@render controls(false)}</div>
    <div class="wide">{@render position()}</div>
    <label class="wide vol" title="Volume">
      <span class="sr">volume</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        value={player.volume()}
        oninput={(e) => player.setVolume(Number((e.currentTarget as HTMLInputElement).value))}
        aria-label="volume"
      />
    </label>
    <button
      type="button"
      class="ctl x"
      onclick={player.stop}
      aria-label="stop and close the player"
    >
      <Icon name="close" size={16} />
    </button>
    {#if player.failed()}<p class="failed" role="alert">{player.failed()}</p>{/if}
  </div>

  {#if open}
    <div class="np-overlay">
      <button
        type="button"
        class="scrim"
        onclick={() => (open = false)}
        aria-label="close"
        tabindex="-1"
      ></button>
      <div class="np" role="dialog" aria-modal="true" aria-label="now playing">
        <header>
          <h2>Now playing</h2>
          <button type="button" class="ctl" onclick={() => (open = false)} aria-label="close">
            <Icon name="close" size={18} />
          </button>
        </header>
        <div class="np-song">
          {#if t.cover}
            <img class="np-art" src={assetUrl(t.cover)} alt="" />
          {/if}
          <div class="np-name">
            <strong>{t.title}</strong>
            {#if by(t)}<span class="by">{by(t)}</span>{/if}
            {#if onopen}
              <button
                type="button"
                class="link"
                onclick={() => {
                  open = false;
                  onopen(t.id);
                }}>Open its note</button
              >
            {/if}
          </div>
        </div>
        {@render position()}
        <div class="np-ctls">{@render controls(true)}</div>
        <h3>Queue</h3>
        <ol class="queue" aria-label="queue">
          {#each player.queue() as q, i (q.id + ':' + i)}
            <li>
              <button
                type="button"
                class="q"
                aria-current={i === player.at() ? 'true' : undefined}
                onclick={() => player.jump(i)}
              >
                <span class="name">
                  <span class="title">{q.title}</span>
                  {#if q.artist}<span class="by">{q.artist}</span>{/if}
                </span>
                {#if q.duration}<span class="time">{clock(q.duration)}</span>{/if}
              </button>
            </li>
          {/each}
        </ol>
      </div>
    </div>
  {/if}
{/if}

<style>
  .player {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    min-height: 3.5rem;
    padding: var(--space-1) var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  .song {
    flex: 1 1 10rem;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .art {
    flex: none;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--surface-hover);
  }
  .name {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .by {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .by,
  .time {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .time {
    font-variant-numeric: tabular-nums;
  }
  .ctls,
  .np-ctls {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .ctl {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .ctl:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .ctl:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .ctl.main {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .ctl.big {
    width: 4rem;
    height: 4rem;
  }
  .x {
    color: var(--text-muted);
  }
  .pos {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
  }
  .pos input {
    flex: 1;
    min-width: 6rem;
  }
  .wide {
    display: none;
  }
  .vol input {
    width: 6rem;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .failed {
    flex-basis: 100%;
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  /* A wide screen has room for where the song is and how loud; a phone opens Now playing for it. */
  @media (min-width: 60rem) {
    .wide {
      display: flex;
      align-items: center;
    }
    .player > .wide:not(.vol) {
      flex: 2 1 16rem;
    }
  }
  .np-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
  }
  .scrim {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .np {
    position: relative;
    width: min(32rem, 100%);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom), var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
    animation: sheet-up var(--dur-med) var(--ease);
  }
  @keyframes sheet-up {
    from {
      transform: translateY(100%);
    }
  }
  .np header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .np h2,
  .np h3 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .np h3 {
    font-size: var(--text-base);
  }
  .np-song {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .np-art {
    width: 6rem;
    height: 6rem;
    border-radius: var(--radius-md);
    object-fit: cover;
  }
  .np-name {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .np-ctls {
    justify-content: center;
    gap: var(--space-4);
  }
  .queue {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .q {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 3rem;
    padding: var(--space-1) 0;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .q .name {
    flex: 1;
  }
  .q[aria-current='true'] .title {
    color: var(--accent);
    font-weight: 600;
  }
</style>

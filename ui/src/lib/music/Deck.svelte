<script lang="ts">
  // **The deck** (owner, 2026-11-14: *"it should look like an mp3, a vintage but full options
  // version … seeing it running, plus volume and sound options"*): the Music view's own player,
  // always on screen there, drawn as a pocket player of its era — a lit display above a row of
  // keys. It drives the one player (`player.svelte.ts`), so what it plays keeps playing elsewhere
  // in the slim bar; while the deck is in view that bar steps aside, and scrolled away it returns.
  //
  // **Lean where it is narrow** (owner, 2026-11-15: *"lean, and fit nicely in mobile devices"*):
  // under 26rem of its own width — a phone, or a narrow pane — one row of five keys and a row of
  // icons; back/forward ten seconds, stop and the words come back where there is room. Its width,
  // not the screen's, decides (a container query), so a narrow pane on a computer is lean too.
  //
  // **Everything a player of the kind had:** the song scrolling across the display, the track
  // number, elapsed or remaining time, a level meter, shuffle, repeat (all / one), back and forward
  // ten seconds, stop, volume and mute, a five-band equaliser with presets and the balance, and the
  // queue. The original look is the app's own, not any product's.
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { assetUrl } from '../ipc';
  import * as player from './player.svelte';

  interface Props {
    /** Open the song's own note in the workspace. */
    onopen?: (id: string) => void;
  }
  let { onopen }: Props = $props();

  let root = $state<HTMLElement | null>(null);
  // In view or not: the slim bar shows only while the deck cannot be seen. With no observer (an
  // older WebView, a test), the deck counts as always in view, as it did first.
  $effect(() => {
    const el = root;
    if (!el) return;
    player.placeDeck(el);
    let release: (() => void) | null = null;
    if (typeof IntersectionObserver === 'undefined') {
      release = untrack(player.showDeck);
      return () => {
        if (release) untrack(release);
        player.placeDeck(null);
      };
    }
    const watch = new IntersectionObserver(
      ([e]) => {
        if (e.isIntersecting && !release) release = untrack(player.showDeck);
        else if (!e.isIntersecting && release) {
          untrack(release);
          release = null;
        }
      },
      { threshold: 0.35 },
    );
    watch.observe(el);
    return () => {
      watch.disconnect();
      if (release) untrack(release);
      player.placeDeck(null);
    };
  });

  const t = $derived(player.now());
  let left = $state(false);
  let soundOpen = $state(false);
  let queueOpen = $state(false);
  const canShape = player.soundAvailable();

  const clock = (secs: number) => {
    const s = Math.max(0, Math.floor(secs));
    return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
  };
  const shownTime = $derived(
    left && player.length() ? `-${clock(player.length() - player.time())}` : clock(player.time()),
  );
  const scroller = $derived(t ? [t.title, t.artist].filter(Boolean).join(' — ') : '');
  // The song scrolls only when it does not fit: measured, since a narrow phone and a wide pane
  // fit very different lengths.
  let strip = $state<HTMLElement | null>(null);
  let width = $state(0);
  let long = $state(false);
  $effect(() => {
    const el = strip;
    if (!el) return;
    width = el.clientWidth;
    if (typeof ResizeObserver === 'undefined') return;
    const watch = new ResizeObserver(() => (width = el.clientWidth));
    watch.observe(el);
    return () => watch.disconnect();
  });
  $effect(() => {
    void scroller;
    void width;
    const el = strip;
    if (!el) return;
    // Measured without the copy that makes the loop: the text alone against the window.
    long = false;
    queueMicrotask(() => (long = el.scrollWidth > el.clientWidth + 1));
  });
  const volSteps = 10;
  const volLevel = $derived(player.muted() ? 0 : Math.round(player.volume() * volSteps));

  // ── the level meter: read off the sound while it plays, on a computer ──────────────────────
  const BARS = 16;
  let bars = $state<number[] | null>(null);
  let frame = 0;
  function tick() {
    bars = player.levels(BARS);
    frame = player.playing() ? requestAnimationFrame(tick) : 0;
  }
  $effect(() => {
    if (player.playing() && typeof requestAnimationFrame === 'function') {
      frame = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(frame);
    }
    bars = bars ? bars.map(() => 0) : null;
  });

  const bandName = (f: number) => (f >= 1000 ? `${f / 1000}k` : String(f));
  const repeatLabel = $derived(
    player.repeat() === 'off'
      ? 'repeat: off'
      : player.repeat() === 'all'
        ? 'repeat: all'
        : 'repeat: one',
  );
</script>

<div class="deck-box">
  <section class="deck" aria-label="music player" bind:this={root}>
    <div class="screen" class:dim={!t}>
      <div class="lcd-row top">
        <span class="state" aria-hidden="true"
          >{player.playing() ? '▶' : t && player.time() > 0 ? '❚❚' : '■'}</span
        >
        <span class="trk">
          {#if t}TRK {String(player.at() + 1).padStart(2, '0')}/{String(
              player.queue().length,
            ).padStart(2, '0')}{:else}NO SONG{/if}
        </span>
        <span class="flags">
          <span class:lit={player.shuffle()}>SHUF</span>
          <span class:lit={player.repeat() !== 'off'}
            >{player.repeat() === 'one' ? 'REP1' : 'REP'}</span
          >
          {#if canShape}<span class:lit={player.preset() !== 'Flat'}>EQ</span>{/if}
        </span>
        <span class="vol-ind" aria-hidden="true" title="Volume">
          {#each { length: volSteps } as _, i (i)}<i class:on={i < volLevel}></i>{/each}
        </span>
      </div>

      <div class="lcd-main">
        {#if t?.cover}
          <img class="art" src={assetUrl(t.cover, 'thumb')} alt="" />
        {/if}
        <div class="words">
          {#if t}<span class="sr" aria-live="polite">now playing: {t.title}</span>{/if}
          <div class="marquee" aria-hidden={t ? 'true' : undefined} bind:this={strip}>
            {#if t}
              <span class:roll={long}
                >{scroller}{#if long}<span aria-hidden="true"
                    >&nbsp;&nbsp;★&nbsp;&nbsp;{scroller}&nbsp;&nbsp;★&nbsp;&nbsp;</span
                  >{/if}</span
              >
            {:else}
              <span>READY — PICK A SONG BELOW</span>
            {/if}
          </div>
          <div class="sub">
            {#if t}{t.album || ' '}{#if player.preset() !== 'Flat' && canShape}<span class="eqname">
                  {player.preset().toUpperCase()}</span
                >{/if}{/if}
          </div>
        </div>
      </div>

      {#if bars}
        <div class="meter" aria-hidden="true">
          {#each bars as b, i (i)}<i style:height="{Math.max(6, Math.round(b * 100))}%"></i>{/each}
        </div>
      {/if}

      <div class="lcd-row time">
        <button
          type="button"
          class="clock"
          onclick={() => (left = !left)}
          disabled={!t}
          aria-label={left ? 'time left — show time played' : 'time played — show time left'}
          title="Time played / time left">{t ? shownTime : '--:--'}</button
        >
        <input
          class="pos"
          type="range"
          min="0"
          max={Math.max(1, Math.round(player.length()))}
          step="1"
          value={Math.round(player.time())}
          disabled={!t}
          style:--done="{player.length() ? (player.time() / player.length()) * 100 : 0}%"
          oninput={(e) => player.seek(Number((e.currentTarget as HTMLInputElement).value))}
          aria-label="position in the song"
        />
        <span class="total">{t && player.length() ? clock(player.length()) : '--:--'}</span>
      </div>
    </div>

    {#if player.failed()}<p class="failed" role="alert">{player.failed()}</p>{/if}

    <div class="keys">
      <button
        type="button"
        class="key small"
        class:lit={player.shuffle()}
        onclick={player.toggleShuffle}
        aria-pressed={player.shuffle()}
        aria-label="shuffle"
        title="Shuffle"><Icon name="shuffle" size={16} /></button
      >
      <button
        type="button"
        class="key"
        onclick={player.previous}
        disabled={!t}
        aria-label="previous song"
        title="Previous"><Icon name="skip-back" size={18} /></button
      >
      <button
        type="button"
        class="key extra"
        onclick={() => player.skip(-10)}
        disabled={!t}
        aria-label="back 10 seconds"
        title="Back 10 seconds"><Icon name="rewind" size={18} /></button
      >
      <button
        type="button"
        class="key main"
        onclick={player.toggle}
        disabled={!t}
        aria-label={player.playing() ? 'pause' : 'play'}
        title={player.playing() ? 'Pause' : 'Play'}
        ><Icon name={player.playing() ? 'pause' : 'play'} size={24} /></button
      >
      <button
        type="button"
        class="key extra"
        onclick={player.halt}
        disabled={!t}
        aria-label="stop"
        title="Stop"><Icon name="stop" size={16} /></button
      >
      <button
        type="button"
        class="key extra"
        onclick={() => player.skip(10)}
        disabled={!t}
        aria-label="forward 10 seconds"
        title="Forward 10 seconds"><Icon name="fast-forward" size={18} /></button
      >
      <button
        type="button"
        class="key"
        onclick={player.next}
        disabled={!t || !player.hasNext()}
        aria-label="next song"
        title="Next"><Icon name="skip-forward" size={18} /></button
      >
      <button
        type="button"
        class="key small"
        class:lit={player.repeat() !== 'off'}
        onclick={player.cycleRepeat}
        aria-label={repeatLabel}
        title="Repeat: off, all, one"
        ><Icon name="repeat" size={16} />{#if player.repeat() === 'one'}<b class="one">1</b
          >{/if}</button
      >
    </div>

    <div class="dials">
      <button
        type="button"
        class="key small"
        onclick={player.toggleMute}
        aria-pressed={player.muted()}
        aria-label={player.muted() ? 'unmute' : 'mute'}
        title={player.muted() ? 'Unmute' : 'Mute'}
        ><Icon
          name={player.muted() || player.volume() === 0 ? 'mute' : 'volume'}
          size={16}
        /></button
      >
      <input
        class="vol"
        type="range"
        min="0"
        max="1"
        step="0.05"
        value={player.muted() ? 0 : player.volume()}
        style:--done="{(player.muted() ? 0 : player.volume()) * 100}%"
        oninput={(e) => player.setVolume(Number((e.currentTarget as HTMLInputElement).value))}
        aria-label="volume"
      />
      {#if canShape}
        <button
          type="button"
          class="pill"
          class:lit={soundOpen}
          onclick={() => (soundOpen = !soundOpen)}
          aria-expanded={soundOpen}
          aria-label="sound settings"
          ><Icon name="sliders" size={16} /><span class="lbl">Sound</span></button
        >
      {/if}
      <button
        type="button"
        class="pill"
        class:lit={queueOpen}
        onclick={() => (queueOpen = !queueOpen)}
        aria-expanded={queueOpen}
        aria-label="queue"
        ><Icon name="list" size={16} /><span class="lbl">Queue</span
        >{#if player.queue().length}<span class="badge">{player.queue().length}</span>{/if}</button
      >
      {#if t && onopen}
        <button
          type="button"
          class="pill"
          onclick={() => onopen(t.id)}
          aria-label="open the song's note"
          ><Icon name="pen" size={16} /><span class="lbl">Note</span></button
        >
      {/if}
    </div>

    {#if soundOpen && canShape}
      <div class="panel sound" role="group" aria-label="equaliser">
        <div class="presets">
          <label>
            <span>Preset</span>
            <select
              value={player.preset()}
              onchange={(e) => player.usePreset((e.currentTarget as HTMLSelectElement).value)}
              aria-label="equaliser preset"
            >
              {#each Object.keys(player.PRESETS) as p (p)}<option value={p}>{p}</option>{/each}
              {#if player.preset() === 'Custom'}<option value="Custom" disabled>Custom</option>{/if}
            </select>
          </label>
          <button type="button" class="pill" onclick={() => player.usePreset('Flat')}>Reset</button>
        </div>
        <div class="bands">
          {#each player.BANDS as f, i (f)}
            <label class="band">
              <span class="db">{player.eq()[i] > 0 ? '+' : ''}{player.eq()[i]}</span>
              <input
                type="range"
                min={-player.GAIN_MAX}
                max={player.GAIN_MAX}
                step="1"
                value={player.eq()[i]}
                oninput={(e) =>
                  player.setBand(i, Number((e.currentTarget as HTMLInputElement).value))}
                aria-label={`${bandName(f)} hertz`}
              />
              <span class="hz">{bandName(f)}</span>
            </label>
          {/each}
        </div>
        <label class="balance">
          <span>L</span>
          <input
            type="range"
            min="-1"
            max="1"
            step="0.1"
            value={player.balance()}
            oninput={(e) => player.setBalance(Number((e.currentTarget as HTMLInputElement).value))}
            ondblclick={() => player.setBalance(0)}
            aria-label="balance"
          />
          <span>R</span>
        </label>
      </div>
    {/if}

    {#if queueOpen}
      <div class="panel">
        {#if !player.queue().length}
          <p class="quiet">The queue is empty. Press a song below to start one.</p>
        {:else}
          <ol class="queue" aria-label="queue">
            {#each player.queue() as q, i (q.id + ':' + i)}
              <li>
                <button
                  type="button"
                  class="q"
                  aria-current={i === player.at() ? 'true' : undefined}
                  onclick={() => player.jump(i)}
                >
                  <span class="n">{i + 1}</span>
                  <span class="qt"
                    >{q.title}{#if q.artist}<span class="by"> · {q.artist}</span>{/if}</span
                  >
                  {#if q.duration}<span class="len">{clock(q.duration)}</span>{/if}
                </button>
                {#if i !== player.at()}
                  <button
                    type="button"
                    class="rm"
                    onclick={() => player.unqueue(i)}
                    aria-label={`take ${q.title} out of the queue`}
                    title="Take out of the queue"><Icon name="close" size={14} /></button
                  >
                {/if}
              </li>
            {/each}
          </ol>
          <button type="button" class="pill" onclick={player.stop}>Clear the queue</button>
        {/if}
      </div>
    {/if}
  </section>
</div>

<style>
  .deck-box {
    container: deck / inline-size;
  }
  .deck {
    --lcd-bg: #b9cf98;
    --lcd-bg2: #a3bd7f;
    --lcd-ink: #1d2812;
    --lcd-ghost: rgb(29 40 18 / 0.18);
    --body: #2c2f34;
    --body2: #1b1d21;
    --key: #e9e6df;
    --key2: #c9c5bb;
    --key-ink: #2a2a2a;
    --glow: #ffb547;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    width: 100%;
    max-width: 30rem;
    margin: 0 auto;
    box-sizing: border-box;
    padding: 0.75rem;
    border-radius: 1rem;
    background:
      linear-gradient(160deg, rgb(255 255 255 / 0.08), transparent 40%),
      linear-gradient(180deg, var(--body), var(--body2));
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.12) inset,
      0 -2px 0 rgb(0 0 0 / 0.4) inset,
      0 6px 14px rgb(0 0 0 / 0.2);
    color: #ddd;
  }
  button,
  input,
  select {
    font: inherit;
    color: inherit;
  }

  /* ── the display ─────────────────────────────────────────────────────────── */
  .screen {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.45rem 0.6rem;
    border-radius: 0.5rem;
    background:
      repeating-linear-gradient(0deg, rgb(0 0 0 / 0.025) 0 1px, transparent 1px 3px),
      linear-gradient(180deg, var(--lcd-bg), var(--lcd-bg2));
    box-shadow:
      0 0 0 3px #111 inset,
      0 3px 8px rgb(0 0 0 / 0.45) inset;
    color: var(--lcd-ink);
    font-family: ui-monospace, 'SFMono-Regular', Menlo, Consolas, 'Liberation Mono', monospace;
    font-variant-numeric: tabular-nums;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    overflow: hidden;
  }
  .screen.dim {
    --lcd-bg: #a9b993;
    --lcd-bg2: #97aa80;
  }
  .lcd-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.72rem;
  }
  .state {
    width: 1.1rem;
  }
  .trk {
    flex: 1;
    white-space: nowrap;
  }
  .flags {
    display: flex;
    gap: 0.35rem;
  }
  .flags span {
    padding: 0 0.15rem;
    color: var(--lcd-ghost);
  }
  .flags span.lit {
    color: var(--lcd-ink);
    outline: 1px solid var(--lcd-ink);
    border-radius: 2px;
  }
  .vol-ind {
    display: flex;
    align-items: flex-end;
    gap: 1px;
    height: 0.75rem;
  }
  .vol-ind i {
    width: 3px;
    background: var(--lcd-ghost);
  }
  .vol-ind i:nth-child(n) {
    height: 40%;
  }
  .vol-ind i:nth-child(n + 4) {
    height: 60%;
  }
  .vol-ind i:nth-child(n + 7) {
    height: 80%;
  }
  .vol-ind i:nth-child(n + 9) {
    height: 100%;
  }
  .vol-ind i.on {
    background: var(--lcd-ink);
  }
  .lcd-main {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.4rem;
  }
  .art {
    flex: none;
    width: 2.4rem;
    height: 2.4rem;
    object-fit: cover;
    border-radius: 2px;
    filter: grayscale(1) sepia(0.6) hue-rotate(40deg) contrast(1.1);
    mix-blend-mode: multiply;
  }
  .words {
    flex: 1;
    min-width: 0;
  }
  .marquee {
    overflow: hidden;
    white-space: nowrap;
    font-size: 1rem;
    font-weight: 700;
    line-height: 1.35;
  }
  .marquee > span {
    display: inline-block;
  }
  .marquee .roll {
    animation: roll 14s linear infinite;
  }
  @keyframes roll {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(-50%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .marquee .roll {
      animation: none;
    }
  }
  .sub {
    font-size: 0.75rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    min-height: 1em;
  }
  .eqname {
    margin-left: 0.5rem;
    opacity: 0.7;
  }
  .meter {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 1rem;
  }
  .meter i {
    flex: 1;
    background: repeating-linear-gradient(0deg, var(--lcd-ink) 0 2px, transparent 2px 3px);
    transition: height 60ms linear;
  }
  .time {
    font-size: 0.95rem;
  }
  .clock {
    padding: 0;
    border: 0;
    background: none;
    font-weight: 700;
    font-size: 1.1rem;
    letter-spacing: 0.06em;
    cursor: pointer;
    min-width: 3.9rem;
    text-align: left;
  }
  .total {
    font-size: 0.75rem;
  }
  .pos {
    flex: 1;
    min-width: 4rem;
  }

  /* ranges: a thin groove with what is done filled in */
  input[type='range'] {
    -webkit-appearance: none;
    appearance: none;
    height: 1.5rem;
    background: transparent;
    cursor: pointer;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(
      90deg,
      var(--fill, currentColor) var(--done, 50%),
      var(--groove, rgb(0 0 0 / 0.25)) var(--done, 50%)
    );
  }
  input[type='range']::-moz-range-track {
    height: 4px;
    border-radius: 2px;
    background: var(--groove, rgb(0 0 0 / 0.25));
  }
  input[type='range']::-moz-range-progress {
    height: 4px;
    border-radius: 2px;
    background: var(--fill, currentColor);
  }
  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 0.8rem;
    height: 0.8rem;
    margin-top: -0.25rem;
    border-radius: 2px;
    background: var(--thumb, currentColor);
  }
  input[type='range']::-moz-range-thumb {
    width: 0.8rem;
    height: 0.8rem;
    border: 0;
    border-radius: 2px;
    background: var(--thumb, currentColor);
  }
  .pos {
    --fill: var(--lcd-ink);
    --thumb: var(--lcd-ink);
    --groove: var(--lcd-ghost);
  }
  input:disabled {
    cursor: default;
    opacity: 0.5;
  }

  /* ── the keys ────────────────────────────────────────────────────────────── */
  .keys {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.25rem;
  }
  .key {
    position: relative;
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: linear-gradient(180deg, var(--key), var(--key2));
    color: var(--key-ink);
    box-shadow:
      0 2px 0 #7d7a72,
      0 2px 4px rgb(0 0 0 / 0.35);
    cursor: pointer;
    transition:
      transform 60ms,
      box-shadow 60ms;
  }
  .key:active:not(:disabled) {
    transform: translateY(2px);
    box-shadow:
      0 0 0 #7d7a72,
      0 1px 2px rgb(0 0 0 / 0.4);
  }
  .key:disabled {
    opacity: 0.45;
    cursor: default;
  }
  /* Touch-sized everywhere: a small key is only drawn lighter. */
  .key.small {
    background: linear-gradient(180deg, #d9d5cc, #b9b4a8);
  }
  .extra {
    display: none;
  }
  .key.main {
    width: 3.4rem;
    height: 3.4rem;
    background: linear-gradient(180deg, #fbfaf6, #d8d3c6);
    box-shadow:
      0 0 0 3px #44474d,
      0 2px 0 5px #16171a,
      0 6px 10px rgb(0 0 0 / 0.45);
  }
  .key.lit {
    color: #b25d00;
    box-shadow:
      0 2px 0 #7d7a72,
      0 0 0 2px var(--glow),
      0 0 10px rgb(255 181 71 / 0.6);
  }
  .one {
    position: absolute;
    right: 0.35rem;
    bottom: 0.3rem;
    font-size: 0.6rem;
  }

  .dials {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .vol {
    flex: 1;
    min-width: 4rem;
    --fill: var(--glow);
    --thumb: #eee;
    --groove: rgb(255 255 255 / 0.15);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    min-width: 2.75rem;
    min-height: 2.75rem;
    padding: 0 0.6rem;
    box-sizing: border-box;
    border: 1px solid rgb(255 255 255 / 0.15);
    border-radius: 999px;
    background: rgb(255 255 255 / 0.06);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .pill :global(svg) {
    flex: none;
  }
  .pill:hover {
    background: rgb(255 255 255 / 0.12);
  }
  .pill.lit {
    border-color: var(--glow);
    color: var(--glow);
  }

  .lbl {
    display: none;
  }
  .badge {
    min-width: 1rem;
    padding: 0 0.3rem;
    border-radius: 999px;
    background: var(--glow);
    color: #222;
    font-size: 0.7rem;
    font-weight: 700;
    line-height: 1.2rem;
    text-align: center;
  }

  /* Room to spare (a wide pane, a tablet): the rest of the keys, the words, a larger display. */
  @container deck (min-width: 26rem) {
    .deck {
      gap: 0.75rem;
      padding: 1rem;
      border-radius: 1.25rem;
    }
    .extra {
      display: grid;
    }
    .lbl {
      display: inline;
    }
    .keys {
      justify-content: center;
      gap: 0.45rem;
    }
    .key.main {
      width: 3.75rem;
      height: 3.75rem;
    }
    .marquee {
      font-size: 1.15rem;
    }
    .clock {
      font-size: 1.3rem;
      min-width: 4.5rem;
    }
    .art {
      width: 3rem;
      height: 3rem;
    }
    .meter {
      height: 1.4rem;
    }
    .band input {
      height: 7rem;
    }
  }

  /* A strip, where it is wide (a computer, a wide pane): the display on the left, the keys and the
     dials beside it, the panels opening under both — a component of a stack, not a pocket set. */
  @container deck (min-width: 48rem) {
    .deck {
      display: grid;
      grid-template-columns: minmax(16rem, 1fr) auto;
      grid-template-areas:
        'screen keys'
        'screen dials';
      align-items: center;
      column-gap: 1rem;
      row-gap: 0.5rem;
      max-width: 64rem;
      padding: 0.75rem 1rem;
    }
    .screen {
      grid-area: screen;
      align-self: stretch;
      justify-content: center;
    }
    .keys {
      grid-area: keys;
    }
    .dials {
      grid-area: dials;
    }
    /* Under both, in rows of their own only while there is something to show. */
    .failed,
    .panel {
      grid-column: 1 / -1;
    }
    .panel {
      max-height: 18rem;
      overflow-y: auto;
    }
    .key.main {
      width: 3.4rem;
      height: 3.4rem;
    }
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    border-radius: 0.8rem;
    background: rgb(0 0 0 / 0.3);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.5) inset;
  }
  .presets {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .presets label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  select {
    min-height: 2.25rem;
    padding: 0 0.5rem;
    border: 1px solid rgb(255 255 255 / 0.2);
    border-radius: 0.4rem;
    background: #22252a;
  }
  .bands {
    display: flex;
    justify-content: space-around;
  }
  .band {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
  }
  .band input {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 1.5rem;
    height: 5rem;
    --fill: var(--glow);
    --thumb: #eee;
    --groove: rgb(255 255 255 / 0.15);
  }
  .band input::-webkit-slider-runnable-track {
    width: 4px;
    height: auto;
    background: rgb(255 255 255 / 0.15);
  }
  .band input::-webkit-slider-thumb {
    margin-top: 0;
    margin-left: -0.25rem;
  }
  .db {
    color: var(--glow);
  }
  .hz {
    color: #aaa;
  }
  .balance {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .balance input {
    flex: 1;
    --fill: rgb(255 255 255 / 0.15);
    --thumb: var(--glow);
    --groove: rgb(255 255 255 / 0.15);
  }

  .queue {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 16rem;
    overflow-y: auto;
  }
  .queue li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid rgb(255 255 255 / 0.08);
  }
  .q {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.5rem;
    padding: 0 0.25rem;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .q:hover {
    background: rgb(255 255 255 / 0.06);
  }
  .q[aria-current='true'] {
    color: var(--glow);
    font-weight: 600;
  }
  .n {
    width: 1.5rem;
    color: #999;
    font-variant-numeric: tabular-nums;
    font-size: var(--text-sm);
  }
  .qt {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .by,
  .len {
    color: #999;
    font-size: var(--text-sm);
  }
  .rm {
    width: 2.25rem;
    height: 2.25rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: none;
    color: #999;
    cursor: pointer;
  }
  .rm:hover {
    background: rgb(255 255 255 / 0.08);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .quiet {
    margin: 0;
    color: #aaa;
    font-size: var(--text-sm);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: #ff9b8a;
  }
</style>

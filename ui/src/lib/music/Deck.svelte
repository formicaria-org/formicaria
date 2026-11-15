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

{#snippet clockButton()}
  <button
    type="button"
    class="clock"
    onclick={() => (left = !left)}
    disabled={!t}
    aria-label={left ? 'time left — show time played' : 'time played — show time left'}
    title="Time played / time left">{t ? shownTime : '--:--'}</button
  >
{/snippet}

{#snippet position()}
  {@const done = player.length() ? (player.time() / player.length()) * 100 : 0}
  <span class="pos-wrap">
    <input
      class="pos"
      type="range"
      min="0"
      max={Math.max(1, Math.round(player.length()))}
      step="1"
      value={Math.round(player.time())}
      disabled={!t}
      style:--done="{done}%"
      oninput={(e) => player.seek(Number((e.currentTarget as HTMLInputElement).value))}
      aria-label="position in the song"
    />
    {#if player.skin() === 'pixel'}
      <!-- A note that runs along the ground as the song goes, hopping while it plays. -->
      <i class="runner" class:hop={player.playing()} style:left="{done}%" aria-hidden="true"></i>
    {/if}
  </span>
{/snippet}

<div class="deck-box">
  <section class="deck skin-{player.skin()}" aria-label="music player" bind:this={root}>
    <div class="screen" class:dim={!t}>
      {#if t}<span class="sr" aria-live="polite">now playing: {t.title}</span>{/if}
      {#if player.skin() === 'arcade'}
        <!-- An arcade fighter's top line: who, the time on the clock, and two bars. -->
        <div class="fight">
          <div class="side">
            <span class="who" aria-hidden="true">{t ? t.artist || 'Unknown' : 'Player 1'}</span>
            {@render position()}
          </div>
          <div class="mid">
            {@render clockButton()}
            <span class="vs" aria-hidden="true">VS</span>
          </div>
          <div class="side right">
            <span class="who" aria-hidden="true">{t ? t.title : 'Insert song'}</span>
            <span class="hp" aria-hidden="true"
              ><i style:width="{player.muted() ? 0 : player.volume() * 100}%"></i></span
            >
          </div>
        </div>
        <div class="round" aria-hidden="true">
          {#if t}ROUND {String(player.at() + 1).padStart(2, '0')}<small>
              / {String(player.queue().length).padStart(2, '0')}</small
            >{:else}PICK A SONG BELOW{/if}
        </div>
        <div class="arcade-flags" aria-hidden="true">
          <span class:lit={player.shuffle()}>SHUFFLE</span>
          <span class:lit={player.repeat() !== 'off'}
            >{player.repeat() === 'one' ? 'REPEAT 1' : 'REPEAT'}</span
          >
          {#if canShape}<span class:lit={player.preset() !== 'Flat'}>EQ</span>{/if}
        </div>
        {#if bars}
          <div class="meter" aria-hidden="true">
            {#each bars as b, i (i)}<i style:height="{Math.max(6, Math.round(b * 100))}%"
              ></i>{/each}
          </div>
        {/if}
      {:else}
        {#if player.skin() === 'pixel'}
          <!-- Sky and two clouds of the 8-bit skin, behind the words. -->
          <span class="sky" aria-hidden="true"><i class="cloud a"></i><i class="cloud b"></i></span>
        {/if}
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
              {#if t}{t.album || ' '}{#if player.preset() !== 'Flat' && canShape}<span
                    class="eqname"
                  >
                    {player.preset().toUpperCase()}</span
                  >{/if}{/if}
            </div>
          </div>
        </div>

        {#if bars}
          <div class="meter" aria-hidden="true">
            {#each bars as b, i (i)}<i style:height="{Math.max(6, Math.round(b * 100))}%"
              ></i>{/each}
          </div>
        {/if}

        <div class="lcd-row time">
          {@render clockButton()}
          {@render position()}
          <span class="total">{t && player.length() ? clock(player.length()) : '--:--'}</span>
        </div>
      {/if}
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
      <button
        type="button"
        class="pill"
        onclick={player.cycleSkin}
        aria-label={`skin: ${player.skinName()}`}
        title="Change the look: Classic LCD, 8-bit, Arcade"
        ><Icon name="palette" size={16} /><span class="lbl">{player.skinName()}</span></button
      >
      {#if t && onopen}
        <button
          type="button"
          class="pill note-pill"
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
  .pos-wrap {
    position: relative;
    flex: 1;
    min-width: 4rem;
    display: flex;
    align-items: center;
  }
  .pos {
    flex: 1;
    min-width: 0;
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

  /* A narrow phone: the note is one ⋯ away in the list, so the row of dials stays one row. */
  @container deck (max-width: 22rem) {
    .note-pill {
      display: none;
    }
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
  /* ══ Skins (owner, 2026-11-15: an old game's look). Original, CSS only: no font, picture or sound
     of any game. The keys, the order and every label stay the same in each. ══════════════════ */

  /* ── 8-bit: bricks, a sky with clouds, hard pixel edges, a note running along the ground ─── */
  .skin-pixel {
    --lcd-ink: #fff;
    --lcd-ghost: rgb(255 255 255 / 0.35);
    --glow: #ffd23f;
    border-radius: 0;
    background-color: #b4521e;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='32' height='32' shape-rendering='crispEdges'%3E%3Crect width='32' height='32' fill='%23b4521e'/%3E%3Cpath d='M0 0h32v2H0zM0 16h32v2H0zM0 0h2v16H0zM16 16h2v16h-2z' fill='%235e2a0c'/%3E%3Cpath d='M2 2h14v2H2zM18 18h14v2H18zM2 18h14v2H2z' fill='%23d9733a'/%3E%3C/svg%3E");
    background-size: 32px 32px;
    border: 4px solid #000;
    box-shadow: none;
    clip-path: polygon(
      0 4px,
      4px 4px,
      4px 0,
      calc(100% - 4px) 0,
      calc(100% - 4px) 4px,
      100% 4px,
      100% calc(100% - 4px),
      calc(100% - 4px) calc(100% - 4px),
      calc(100% - 4px) 100%,
      4px 100%,
      4px calc(100% - 4px),
      0 calc(100% - 4px)
    );
    image-rendering: pixelated;
  }
  .skin-pixel .screen {
    position: relative;
    border-radius: 0;
    background: linear-gradient(
      180deg,
      #64b4ff 0 calc(100% - 14px),
      #3cb043 0 calc(100% - 10px),
      #8a4b1f 0
    );
    box-shadow: 0 0 0 4px #000;
    text-shadow: 2px 2px 0 #000;
    font-weight: 700;
    letter-spacing: 0.08em;
    overflow: hidden;
  }
  .skin-pixel .screen > :not(.sky) {
    position: relative;
  }
  .skin-pixel .screen.dim {
    background: linear-gradient(
      180deg,
      #4a8fd1 0 calc(100% - 14px),
      #2f8a35 0 calc(100% - 10px),
      #6e3b18 0
    );
  }
  .skin-pixel .lcd-row.time {
    padding-bottom: 10px;
  }
  .sky {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  /* A cloud: one 4px block, drawn out to a cloud by its shadows. */
  .cloud {
    position: absolute;
    width: 4px;
    height: 4px;
    background: #fff;
    box-shadow:
      4px 0 #fff,
      8px 0 #fff,
      12px 0 #fff,
      -4px 4px #fff,
      0 4px #fff,
      4px 4px #fff,
      8px 4px #fff,
      12px 4px #fff,
      16px 4px #fff,
      -8px 8px #fff,
      -4px 8px #fff,
      0 8px #fff,
      4px 8px #fff,
      8px 8px #fff,
      12px 8px #fff,
      16px 8px #fff,
      20px 8px #fff;
    opacity: 0.85;
    animation: drift 40s linear infinite;
  }
  .cloud.a {
    top: 6px;
    left: 55%;
  }
  .cloud.b {
    top: 22px;
    left: 20%;
    animation-duration: 60s;
    animation-delay: -20s;
    opacity: 0.6;
  }
  @keyframes drift {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(-120vw);
    }
  }
  .skin-pixel .flags span.lit {
    outline: 2px solid #fff;
    border-radius: 0;
    color: #fff;
  }
  .skin-pixel .art {
    border-radius: 0;
    filter: contrast(1.2) saturate(1.3);
    mix-blend-mode: normal;
    box-shadow: 0 0 0 2px #000;
  }
  .skin-pixel .meter i {
    background: repeating-linear-gradient(0deg, #ffd23f 0 4px, transparent 4px 6px);
    box-shadow: 1px 1px 0 #000;
  }
  .skin-pixel .vol-ind i {
    width: 4px;
  }
  .skin-pixel .vol-ind i.on {
    background: #ffd23f;
    box-shadow: 1px 1px 0 #000;
  }
  .skin-pixel .pos {
    --fill: #ffd23f;
    --thumb: #fff;
    --groove: rgb(0 0 0 / 0.35);
  }
  .skin-pixel input[type='range']::-webkit-slider-runnable-track {
    border-radius: 0;
  }
  .skin-pixel input[type='range']::-webkit-slider-thumb {
    border-radius: 0;
    box-shadow: 0 0 0 2px #000;
  }
  .skin-pixel input[type='range']::-moz-range-thumb {
    border-radius: 0;
    box-shadow: 0 0 0 2px #000;
  }
  /* The runner: an eighth note in 3px blocks, drawn by its shadows; it hops while the song plays. */
  .runner {
    position: absolute;
    bottom: calc(50% + 2px);
    width: 3px;
    height: 3px;
    margin-left: -6px;
    background: #000;
    box-shadow:
      3px 0 #000,
      0 3px #000,
      3px 3px #000,
      6px -3px #000,
      6px -6px #000,
      6px -9px #000,
      6px -12px #000,
      9px -12px #000,
      12px -9px #000,
      12px -6px #000,
      -3px 3px #000;
    pointer-events: none;
    transition: left 0.25s steps(4);
  }
  .runner.hop {
    animation: hop 0.5s steps(2) infinite;
  }
  @keyframes hop {
    50% {
      transform: translateY(-5px);
    }
  }
  .skin-pixel .key,
  .skin-pixel .pill {
    border-radius: 0;
    border: 3px solid #000;
    background: #ffd23f;
    color: #000;
    box-shadow:
      inset -3px -3px 0 #c98f00,
      inset 3px 3px 0 #fff2a8,
      0 4px 0 #000;
  }
  .skin-pixel .key.main {
    background: #e83b2e;
    color: #fff;
    box-shadow:
      inset -4px -4px 0 #9c1d14,
      inset 4px 4px 0 #ff8a7f,
      0 5px 0 #000;
  }
  .skin-pixel .key:active:not(:disabled) {
    transform: translateY(4px);
    box-shadow:
      inset -3px -3px 0 #c98f00,
      inset 3px 3px 0 #fff2a8,
      0 0 0 #000;
  }
  .skin-pixel .key.lit,
  .skin-pixel .pill.lit {
    background: #5cd65c;
    color: #000;
    box-shadow:
      inset -3px -3px 0 #2e8b2e,
      inset 3px 3px 0 #c8ffc8,
      0 4px 0 #000;
  }
  .skin-pixel .vol {
    --fill: #ffd23f;
    --thumb: #fff;
    --groove: rgb(0 0 0 / 0.4);
  }
  .skin-pixel .panel {
    border-radius: 0;
    background: rgb(0 0 0 / 0.65);
    box-shadow: 0 0 0 3px #000;
  }
  .skin-pixel .badge {
    border-radius: 0;
    background: #e83b2e;
    color: #fff;
  }

  /* ── Arcade: a night stage, two bars and a clock, neon, round cabinet buttons ─────────────── */
  .skin-arcade {
    --glow: #ffe14d;
    background:
      linear-gradient(transparent 55%, rgb(255 43 214 / 0.18)),
      repeating-linear-gradient(90deg, rgb(255 43 214 / 0.14) 0 1px, transparent 1px 28px),
      radial-gradient(ellipse at 50% 0%, #3b1170 0%, transparent 70%),
      linear-gradient(180deg, #170a2e, #0b0614);
    box-shadow:
      0 0 0 2px #ff2bd6,
      0 0 18px rgb(255 43 214 / 0.45),
      0 6px 14px rgb(0 0 0 / 0.4);
    color: #f4f0ff;
  }
  .skin-arcade .screen {
    gap: 0.35rem;
    padding: 0.5rem 0.6rem 0.6rem;
    background: linear-gradient(180deg, rgb(0 0 0 / 0.55), rgb(20 0 40 / 0.55));
    box-shadow:
      0 0 0 2px #22e5ff,
      0 0 12px rgb(34 229 255 / 0.5) inset;
    color: #fff;
    font-family: Impact, 'Arial Black', 'Helvetica Neue', system-ui, sans-serif;
    font-style: italic;
    letter-spacing: 0.04em;
  }
  .fight {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: 0.5rem;
  }
  .side {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .side.right {
    align-items: flex-end;
    text-align: right;
  }
  .who {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 0.8rem;
    color: #ffe14d;
    text-shadow: 0 0 6px rgb(255 225 77 / 0.6);
  }
  .side .pos-wrap {
    width: 100%;
  }
  /* The song's bar: what is left of it, yellow, worn down in red as it plays. */
  .skin-arcade .pos {
    --fill: #e8213b;
    --groove: #ffe14d;
    --thumb: #fff;
    height: 1.1rem;
  }
  .skin-arcade .pos::-webkit-slider-runnable-track {
    height: 0.7rem;
    border-radius: 0;
    box-shadow: 0 0 0 2px #fff;
  }
  .skin-arcade .pos::-moz-range-track {
    height: 0.7rem;
    border-radius: 0;
  }
  .skin-arcade .pos::-moz-range-progress {
    height: 0.7rem;
    border-radius: 0;
  }
  .skin-arcade .pos::-webkit-slider-thumb {
    width: 0.35rem;
    height: 0.9rem;
    margin-top: -0.1rem;
    border-radius: 0;
  }
  .skin-arcade .pos::-moz-range-thumb {
    width: 0.35rem;
    height: 0.9rem;
    border-radius: 0;
  }
  /* The other bar is the volume, filled from the right as a second player's would be. */
  .hp {
    display: flex;
    justify-content: flex-end;
    width: 100%;
    height: 0.7rem;
    margin: 0.2rem 0;
    background: #e8213b;
    box-shadow: 0 0 0 2px #fff;
  }
  .hp i {
    display: block;
    height: 100%;
    background: #ffe14d;
  }
  .mid {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .skin-arcade .clock {
    min-width: 0;
    font-size: 1.5rem;
    font-weight: 900;
    color: #fff;
    text-align: center;
    -webkit-text-stroke: 1px #e8213b;
    text-shadow: 0 0 8px rgb(255 43 214 / 0.8);
  }
  .vs {
    font-size: 0.7rem;
    font-weight: 900;
    color: #22e5ff;
  }
  .round {
    text-align: center;
    font-size: 1.25rem;
    font-weight: 900;
    background: linear-gradient(180deg, #fff6b0, #ffb300 55%, #ff5a00);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    filter: drop-shadow(2px 2px 0 #3a0a52);
  }
  .round small {
    font-size: 0.75em;
  }
  .arcade-flags {
    display: flex;
    justify-content: center;
    gap: 0.75rem;
    font-size: 0.65rem;
    letter-spacing: 0.12em;
  }
  .arcade-flags span {
    color: rgb(255 255 255 / 0.25);
  }
  .arcade-flags span.lit {
    color: #22e5ff;
    text-shadow: 0 0 6px #22e5ff;
  }
  .skin-arcade .meter i {
    background: linear-gradient(0deg, #22e5ff, #ff2bd6);
    border-radius: 1px;
  }
  /* Cabinet buttons: round, domed, each its own colour, set in a dark ring. */
  .skin-arcade .key {
    color: #fff;
    background: radial-gradient(circle at 50% 35%, #6aa8ff, #1f5fd6 60%, #123a85);
    box-shadow:
      0 0 0 3px #0d0d12,
      0 0 0 5px #6b6b78,
      0 4px 8px rgb(0 0 0 / 0.6);
  }
  .skin-arcade .keys .key:nth-child(odd) {
    background: radial-gradient(circle at 50% 35%, #ff9de8, #d61fb0 60%, #7a0f63);
  }
  .skin-arcade .key.small {
    background: radial-gradient(circle at 50% 35%, #fff59a, #f2c200 60%, #8a6d00);
    color: #2a1d00;
  }
  .skin-arcade .key.main {
    background: radial-gradient(circle at 50% 35%, #ff8a8a, #e8213b 60%, #7a0a1a);
    box-shadow:
      0 0 0 3px #0d0d12,
      0 0 0 6px #b9b9c6,
      0 0 16px rgb(232 33 59 / 0.6);
  }
  .skin-arcade .key:active:not(:disabled) {
    transform: translateY(2px) scale(0.97);
  }
  .skin-arcade .key.lit {
    box-shadow:
      0 0 0 3px #0d0d12,
      0 0 0 5px #22e5ff,
      0 0 14px #22e5ff;
  }
  .skin-arcade .pill {
    border-color: #22e5ff;
    background: rgb(34 229 255 / 0.08);
    color: #c9f8ff;
  }
  .skin-arcade .pill.lit {
    border-color: #ffe14d;
    color: #ffe14d;
  }
  .skin-arcade .vol {
    --fill: #ff2bd6;
    --thumb: #fff;
    --groove: rgb(255 255 255 / 0.15);
  }
  .skin-arcade .badge {
    background: #ff2bd6;
    color: #fff;
  }

  @media (prefers-reduced-motion: reduce) {
    .cloud,
    .runner.hop {
      animation: none;
    }
  }
</style>

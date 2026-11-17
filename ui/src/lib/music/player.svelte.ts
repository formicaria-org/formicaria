// **The player is the app's own** (`decisions.md#ui`, 2026-11-13, *a sixth group, Music*, M3): one
// audio element for the whole app, outside every pane, so a song keeps playing while the person
// moves between views and windows. Like the undo strip, it belongs to no group.
//
// **The queue and the settings are device state**, kept in `localStorage` like the board's column
// order — never in the vault. A queue brought back after a restart is shown paused: nothing plays
// unasked.
//
// **The browser's Media Session** gets the song and the controls where there is one: media keys and
// the system's own player on a computer. An Android WebView has none (`music-plan-2026-11-13.md`,
// Part 1), which is M4's native player, later.
//
// **Sound** (the equaliser, the balance and the level meter) runs the element through the browser's
// Web Audio graph. That graph is silent for audio from another origin, which is what the phone's
// file protocol is, so it is only ever built where the song comes from the page's own origin — the
// computer — and `soundAvailable()` says so for the UI to offer it or not.
import { assetPaths, assetUrl } from '../ipc';
import type { ObjectMeta } from '../types';

/** What the player needs of a song: its note, its audio and what to call it. */
export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string;
  /** `sha256:…`, the audio's reference. */
  audio: string;
  cover: string | null;
  /** Whole seconds, from the file's tags; the stream's own length wins once it is known. */
  duration: number | null;
}

/** A song note as a track, or `null` for a note with no audio to play. */
export function trackOf(s: ObjectMeta): Track | null {
  const text = (k: string) => (typeof s.props[k] === 'string' ? (s.props[k] as string) : '');
  const audio = text('audio') || s.assets.find((a) => a.startsWith('sha256:')) || '';
  if (!audio) return null;
  return {
    id: s.id,
    title: s.title ?? '',
    artist: text('artist'),
    album: text('album'),
    audio,
    cover: text('cover') || null,
    duration: typeof s.props.duration_s === 'number' ? s.props.duration_s : null,
  };
}

export type Repeat = 'off' | 'all' | 'one';

/**
 * How the deck looks: the green display it started with, or one of two original skins in the
 * spirit of old games (owner, 2026-11-15) — an 8-bit platformer's and an arcade fighter's. Drawn
 * by CSS alone: no font, picture or sound of any game.
 */
export type Skin = 'lcd' | 'pixel' | 'arcade';
export const SKINS: [Skin, string][] = [
  ['lcd', 'Classic LCD'],
  ['pixel', '8-bit'],
  ['arcade', 'Arcade'],
];

/** The equaliser's bands, in hertz: the classic five of a pocket player. */
export const BANDS = [60, 230, 910, 3600, 14000] as const;
/** How far a band moves, either way, in decibels. */
export const GAIN_MAX = 12;
/** Named settings of the five bands. */
export const PRESETS: Record<string, number[]> = {
  Flat: [0, 0, 0, 0, 0],
  'Bass boost': [7, 4, 0, 0, 0],
  Rock: [5, 2, -2, 3, 5],
  Pop: [-1, 3, 5, 3, -1],
  Jazz: [3, 1, -1, 2, 4],
  Classical: [4, 2, 0, 2, 3],
  Vocal: [-3, 0, 4, 3, -1],
  'Treble boost': [0, 0, 0, 4, 7],
};

const KEY = 'fm-queue';
const SETTINGS = 'fm-player';

const state = $state({
  queue: [] as Track[],
  at: -1,
  playing: false,
  /** Seconds into the song. */
  time: 0,
  /** The song's length in seconds, 0 while unknown. */
  length: 0,
  volume: 1,
  muted: false,
  shuffle: false,
  repeat: 'off' as Repeat,
  /** The order songs come in when shuffled: queue indexes, the current one first. */
  order: [] as number[],
  /** One gain per band of `BANDS`, in decibels. */
  eq: [0, 0, 0, 0, 0] as number[],
  /** −1 all left, 1 all right. */
  balance: 0,
  skin: 'lcd' as Skin,
  /** A sentence to show when a song cannot be played — usually its file is not on this device. */
  failed: '',
  /** How many full-size players are on screen: while one is, the slim bar is not needed. */
  decks: 0,
});

export const queue = () => state.queue;
export const at = () => state.at;
export const now = (): Track | null => state.queue[state.at] ?? null;
export const playing = () => state.playing;
export const time = () => state.time;
export const length = () => state.length || now()?.duration || 0;
export const volume = () => state.volume;
export const muted = () => state.muted;
export const shuffle = () => state.shuffle;
export const repeat = () => state.repeat;
export const eq = () => state.eq;
export const balance = () => state.balance;
export const skin = () => state.skin;
export const skinName = () => SKINS.find(([k]) => k === state.skin)?.[1] ?? 'Classic LCD';
export const failed = () => state.failed;
export const deckShown = () => state.decks > 0;

/** The name of the preset the bands match, or `Custom`. */
export const preset = () =>
  Object.entries(PRESETS).find(([, g]) => g.every((v, i) => v === state.eq[i]))?.[0] ?? 'Custom';

// ── the phone's native player (`PlaybackService`, 2026-11-17) ──────────────────────────────────
// On the phone the music plays in a native media service, so it goes on with the screen off and the
// lock screen, notification, headphones, Bluetooth and the car control it. It holds the whole queue
// (next, previous, shuffle and repeat happen there); this player is its remote and mirrors it.
interface Native {
  setQueue(json: string, index: number, seconds: number, play: boolean): void;
  play(): void;
  pause(): void;
  next(): void;
  previous(): void;
  jump(index: number): void;
  seek(seconds: number): void;
  volume(v: number): void;
  modes(shuffle: boolean, repeat: string): void;
  stop(): void;
}
const native = (): Native | undefined => (globalThis as { __fmPlayer?: Native }).__fmPlayer;
/** The queue the native player holds (its songs' references), to know when it must be told again. */
let handed = '';
const queueKey = () => state.queue.map((t) => t.audio).join('|');

/** Give the native player the queue, from `i` at `seconds`. */
async function handOver(i: number, play: boolean, seconds = 0) {
  const n = native();
  if (!n) return;
  const key = queueKey();
  const refs = state.queue.flatMap((t) => [t.audio, t.cover ?? '']);
  let paths: (string | null)[] = [];
  try {
    paths = await assetPaths(refs);
  } catch {
    paths = [];
  }
  const items = state.queue.map((t, k) => ({
    path: paths[2 * k] ?? '',
    title: t.title,
    artist: t.artist,
    album: t.album,
    cover: paths[2 * k + 1] ?? '',
  }));
  if (!items[i]?.path && state.queue[i]) {
    state.failed = `“${state.queue[i].title}” cannot be played here — its file may not be on this device.`;
    state.playing = false;
    return;
  }
  n.setQueue(JSON.stringify(items), i, seconds, play);
  n.modes(state.shuffle, state.repeat);
  n.volume(state.muted ? 0 : state.volume);
  handed = key;
}
/** The queue changed while the native player plays: it is told again, where it is. */
function resync() {
  if (native() && handed && handed !== queueKey())
    void handOver(state.at, state.playing, state.time);
}
if (typeof window !== 'undefined')
  window.addEventListener('fm-player', (e) => {
    const d = (
      e as CustomEvent<{
        index: number;
        playing: boolean;
        time: number;
        length: number;
        error: string;
      }>
    ).detail;
    if (!d || !handed) return;
    if (d.index >= 0 && d.index < state.queue.length && d.index !== state.at) {
      state.at = d.index;
      describe();
      save();
    }
    state.playing = d.playing;
    state.time = d.time;
    if (d.length > 0) state.length = d.length;
    if (d.error) state.failed = d.error;
  });

let el: HTMLAudioElement | null = null;
/** The one audio element, made on first use: a page that never plays music never has one. */
function audio(): HTMLAudioElement {
  if (el) return el;
  const a = new Audio();
  a.preload = 'metadata';
  a.volume = state.volume;
  a.muted = state.muted;
  a.addEventListener('timeupdate', () => (state.time = a.currentTime));
  a.addEventListener('durationchange', () => {
    if (Number.isFinite(a.duration) && a.duration > 0) state.length = a.duration;
  });
  a.addEventListener('play', () => (state.playing = true));
  a.addEventListener('pause', () => (state.playing = false));
  a.addEventListener('ended', () => {
    state.playing = false;
    if (state.repeat === 'one') {
      a.currentTime = 0;
      start(a);
      return;
    }
    const n = following();
    if (n !== null) go(n, true);
  });
  a.addEventListener('error', () => {
    state.playing = false;
    // The commonest reason by far: the song came from another device and its file did not.
    if (now())
      state.failed = `“${now()!.title}” cannot be played here — its file may not be on this device.`;
  });
  el = a;
  return a;
}

/** Start the element, and say so if the browser refuses (it may, before any press on the page). */
function start(a: HTMLAudioElement) {
  wire(a);
  try {
    const p = a.play() as Promise<void> | undefined;
    p?.catch((e: unknown) => {
      // A newer `src` interrupting an older `play()` is not a failure anyone needs to read.
      if (e instanceof DOMException && e.name === 'AbortError') return;
      state.failed = String(e instanceof Error ? e.message : e);
    });
  } catch (e) {
    state.failed = String(e);
  }
}

// ── the order songs come in ─────────────────────────────────────────────────────────────────────
/** A fresh shuffled order of the queue, with `first` first. */
function shuffled(first: number): number[] {
  const rest = state.queue.map((_, i) => i).filter((i) => i !== first);
  for (let i = rest.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [rest[i], rest[j]] = [rest[j], rest[i]];
  }
  return first >= 0 ? [first, ...rest] : rest;
}
/** The queue index of the song after the current one, or `null` at the end (repeat off). */
function following(): number | null {
  const n = state.queue.length;
  if (!n) return null;
  if (state.shuffle && state.order.length === n) {
    const k = state.order.indexOf(state.at);
    if (k < n - 1) return state.order[k + 1];
    if (state.repeat !== 'all') return null;
    state.order = shuffled(-1);
    return state.order[0];
  }
  if (state.at < n - 1) return state.at + 1;
  return state.repeat === 'all' ? 0 : null;
}
/** The queue index of the song before the current one, or `null` at the start. */
function preceding(): number | null {
  const n = state.queue.length;
  if (state.shuffle && state.order.length === n) {
    const k = state.order.indexOf(state.at);
    return k > 0 ? state.order[k - 1] : null;
  }
  if (state.at > 0) return state.at - 1;
  return state.repeat === 'all' && n ? n - 1 : null;
}
export const hasNext = () => (state.repeat === 'all' ? state.queue.length > 0 : peekNext());
function peekNext() {
  const n = state.queue.length;
  if (state.shuffle && state.order.length === n) return state.order.indexOf(state.at) < n - 1;
  return state.at < n - 1;
}

/** Point the element at the song at `i` and, when asked, play it. */
function go(i: number, play: boolean) {
  state.at = i;
  const t = now();
  if (!t) return;
  if (native()) {
    state.time = 0;
    state.length = t.duration ?? 0;
    state.failed = '';
    // As asked, until the native player says otherwise (its events follow within half a second).
    state.playing = play;
    if (handed === queueKey()) {
      native()!.jump(i);
      if (!play) native()!.pause();
    } else void handOver(i, play);
    describe();
    save();
    return;
  }
  const a = audio();
  a.src = assetUrl(t.audio);
  state.time = 0;
  state.length = t.duration ?? 0;
  state.failed = '';
  if (play) start(a);
  else state.playing = false;
  describe();
  save();
}

/** Play `tracks` from the one at `from`: the list a person pressed in becomes the queue. */
export function playList(tracks: Track[], from: number) {
  if (!tracks[from]) return;
  state.queue = tracks.slice();
  state.order = state.shuffle ? shuffled(from) : [];
  go(from, true);
}

/** Play `tracks` shuffled, from a random one: an album's or a playlist's *Shuffle*. */
export function shuffleList(tracks: Track[]) {
  if (!tracks.length) return;
  state.shuffle = true;
  state.queue = tracks.slice();
  const from = Math.floor(Math.random() * tracks.length);
  state.order = shuffled(from);
  saveSettings();
  go(from, true);
}

/** Put songs straight after the one playing: *Play next*. */
export function playNext(tracks: Track[]) {
  if (!tracks.length) return;
  if (!now()) return playList(tracks, 0);
  const q = state.queue.slice();
  q.splice(state.at + 1, 0, ...tracks);
  state.queue = q;
  if (state.shuffle) {
    // The new songs come next in the shuffled order too, then the rest as they were.
    const added = tracks.map((_, k) => state.at + 1 + k);
    const old = state.order.map((i) => (i > state.at ? i + tracks.length : i));
    const k = old.indexOf(state.at);
    state.order = [...old.slice(0, k + 1), ...added, ...old.slice(k + 1)];
  }
  save();
  resync();
}

/** Put songs at the end of the queue: *Add to queue*. */
export function enqueue(tracks: Track[]) {
  if (!tracks.length) return;
  if (!now()) return playList(tracks, 0);
  const base = state.queue.length;
  state.queue = [...state.queue, ...tracks];
  if (state.shuffle) state.order = [...state.order, ...tracks.map((_, k) => base + k)];
  save();
  resync();
}

/** Take the song at `i` out of the queue. The one playing stays where it is. */
export function unqueue(i: number) {
  if (i === state.at || !state.queue[i]) return;
  state.queue = state.queue.filter((_, k) => k !== i);
  if (i < state.at) state.at -= 1;
  state.order = state.order.filter((k) => k !== i).map((k) => (k > i ? k - 1 : k));
  save();
  resync();
}

export function toggle() {
  const t = now();
  if (!t) return;
  if (native()) {
    state.failed = '';
    if (handed !== queueKey()) void handOver(state.at, true, state.time);
    else if (state.playing) native()!.pause();
    else native()!.play();
    return;
  }
  const a = audio();
  // A queue brought back after a restart, or stopped, has no source yet.
  if (!a.getAttribute('src')) {
    a.src = assetUrl(t.audio);
    if (state.time) a.currentTime = state.time;
  }
  if (a.paused) {
    state.failed = '';
    start(a);
  } else a.pause();
}

/** Stop and go back to the start of the song, keeping the queue: the ■ of a player. */
export function halt() {
  if (native() && handed) {
    native()!.pause();
    native()!.seek(0);
  }
  if (el) {
    el.pause();
    el.currentTime = 0;
  }
  state.time = 0;
  state.playing = false;
}

export function next() {
  if (native() && handed === queueKey()) return native()!.next();
  const n = following();
  if (n !== null) go(n, true);
}

/** Back to the start of the song; to the one before when already near its start, as players do. */
export function previous() {
  if (native() && handed === queueKey()) return native()!.previous();
  const p = preceding();
  if (state.time > 3 || p === null) seek(0);
  else go(p, true);
}

export function jump(i: number) {
  if (state.queue[i]) go(i, true);
}

export function seek(seconds: number) {
  const s = Math.min(Math.max(0, seconds), length() || Infinity);
  if (native() && handed) native()!.seek(s);
  if (el) el.currentTime = s;
  state.time = s;
}

/** Move by `by` seconds, back or forward: ⏪ and ⏩. */
export const skip = (by: number) => seek(state.time + by);

export function setVolume(v: number) {
  state.volume = Math.min(1, Math.max(0, Math.round(v * 100) / 100));
  if (state.volume > 0) state.muted = false;
  if (native() && handed) native()!.volume(state.muted ? 0 : state.volume);
  if (el) {
    el.volume = state.volume;
    el.muted = state.muted;
  }
  saveSettings();
}

export function toggleMute() {
  state.muted = !state.muted;
  if (native() && handed) native()!.volume(state.muted ? 0 : state.volume);
  if (el) el.muted = state.muted;
  saveSettings();
}

export function toggleShuffle() {
  state.shuffle = !state.shuffle;
  state.order = state.shuffle ? shuffled(state.at) : [];
  if (native() && handed) native()!.modes(state.shuffle, state.repeat);
  saveSettings();
}

/** The next look: Classic LCD → 8-bit → Arcade → Classic LCD. */
export function cycleSkin() {
  const i = SKINS.findIndex(([k]) => k === state.skin);
  state.skin = SKINS[(i + 1) % SKINS.length][0];
  saveSettings();
}

/** Off → all → one → off. */
export function cycleRepeat() {
  state.repeat = state.repeat === 'off' ? 'all' : state.repeat === 'all' ? 'one' : 'off';
  if (native() && handed) native()!.modes(state.shuffle, state.repeat);
  saveSettings();
}

/** Stop and empty the queue: the player goes away. */
export function stop() {
  if (native() && handed) {
    native()!.stop();
    handed = '';
  }
  if (el) {
    el.pause();
    el.removeAttribute('src');
    el.load?.();
  }
  state.queue = [];
  state.order = [];
  state.at = -1;
  state.playing = false;
  state.time = 0;
  state.length = 0;
  state.failed = '';
  describe();
  save();
}

let deckEl: HTMLElement | null = null;
/**
 * A full-size player is in view (answers the call that says it no longer is). While one is, the
 * slim bar is not needed; scrolled out of view, the bar comes back so the keys stay in reach.
 */
export function showDeck() {
  state.decks += 1;
  return () => (state.decks -= 1);
}
/** The deck the Music view drew, in view or not: where the slim bar's press goes back to. */
export function placeDeck(el: HTMLElement | null) {
  deckEl = el;
}
/** Bring the deck back into view, when there is one. Answers whether there was. */
export function toDeck(): boolean {
  if (!deckEl?.isConnected) return false;
  deckEl.scrollIntoView?.({ behavior: 'smooth', block: 'start' });
  return true;
}

// ── sound: the equaliser, the balance and the meter ─────────────────────────────────────────────
interface Graph {
  ctx: AudioContext;
  bands: BiquadFilterNode[];
  pan: StereoPannerNode | null;
  meter: AnalyserNode;
}
let graph: Graph | null = null;
let graphFailed = false;

/** Can this device shape the sound? Only for audio from the page's own origin (see the top). */
export function soundAvailable(): boolean {
  if (graphFailed || typeof window === 'undefined') return false;
  const Ctx =
    window.AudioContext ??
    (window as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctx) return false;
  try {
    return new URL(assetUrl('sha256:0'), window.location.href).origin === window.location.origin;
  } catch {
    return false;
  }
}

/** Run the element through the graph, once, from a press (a context needs one to start). */
function wire(a: HTMLAudioElement) {
  if (graph) {
    if (graph.ctx.state === 'suspended') void graph.ctx.resume().catch(() => {});
    return;
  }
  if (!soundAvailable()) return;
  try {
    const Ctx =
      window.AudioContext ??
      (window as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext!;
    const ctx = new Ctx();
    const source = ctx.createMediaElementSource(a);
    const bands = BANDS.map((f, i) => {
      const b = ctx.createBiquadFilter();
      b.type = i === 0 ? 'lowshelf' : i === BANDS.length - 1 ? 'highshelf' : 'peaking';
      b.frequency.value = f;
      b.Q.value = 1;
      b.gain.value = state.eq[i] ?? 0;
      return b;
    });
    const pan = typeof ctx.createStereoPanner === 'function' ? ctx.createStereoPanner() : null;
    if (pan) pan.pan.value = state.balance;
    const meter = ctx.createAnalyser();
    meter.fftSize = 64;
    meter.smoothingTimeConstant = 0.7;
    let node: AudioNode = source;
    for (const b of bands) {
      node.connect(b);
      node = b;
    }
    if (pan) {
      node.connect(pan);
      node = pan;
    }
    node.connect(meter);
    meter.connect(ctx.destination);
    graph = { ctx, bands, pan, meter };
    if (ctx.state === 'suspended') void ctx.resume().catch(() => {});
  } catch {
    // Whatever went wrong, the song plays as it is: sound settings are a nicety, never a wall.
    graphFailed = true;
  }
}

export function setBand(i: number, db: number) {
  const g = Math.max(-GAIN_MAX, Math.min(GAIN_MAX, Math.round(db)));
  state.eq = state.eq.map((v, k) => (k === i ? g : v));
  if (graph?.bands[i]) graph.bands[i].gain.value = g;
  saveSettings();
}

export function usePreset(name: string) {
  const g = PRESETS[name];
  if (!g) return;
  state.eq = g.slice();
  graph?.bands.forEach((b, i) => (b.gain.value = g[i]));
  saveSettings();
}

export function setBalance(v: number) {
  state.balance = Math.max(-1, Math.min(1, Math.round(v * 10) / 10));
  if (graph?.pan) graph.pan.pan.value = state.balance;
  saveSettings();
}

/**
 * The level of `n` bands of the sound now, each 0–1, for the meter; `null` while nothing is
 * wired (a phone, a test, or before the first press of play).
 */
export function levels(n: number): number[] | null {
  if (!graph) return null;
  const bins = new Uint8Array(graph.meter.frequencyBinCount);
  graph.meter.getByteFrequencyData(bins);
  const per = Math.max(1, Math.floor(bins.length / n));
  return Array.from({ length: n }, (_, i) => {
    let m = 0;
    for (let k = i * per; k < Math.min(bins.length, (i + 1) * per); k++) m = Math.max(m, bins[k]);
    return m / 255;
  });
}

// ── kept on this device ─────────────────────────────────────────────────────────────────────────
function save() {
  try {
    if (state.queue.length)
      localStorage.setItem(KEY, JSON.stringify({ queue: state.queue, at: state.at }));
    else localStorage.removeItem(KEY);
  } catch {
    /* no storage: the queue lasts this session */
  }
}

function saveSettings() {
  try {
    localStorage.setItem(
      SETTINGS,
      JSON.stringify({
        volume: state.volume,
        muted: state.muted,
        shuffle: state.shuffle,
        repeat: state.repeat,
        eq: state.eq,
        balance: state.balance,
        skin: state.skin,
      }),
    );
  } catch {
    /* no storage: the settings last this session */
  }
}

/** Bring the last queue and settings back, paused. Called once when the app starts, and by tests. */
export function restore() {
  if (el) {
    el.pause();
    el.removeAttribute('src');
  }
  state.queue = [];
  state.order = [];
  state.at = -1;
  state.playing = false;
  state.time = 0;
  state.length = 0;
  state.failed = '';
  state.volume = 1;
  state.muted = false;
  state.shuffle = false;
  state.repeat = 'off';
  state.eq = [0, 0, 0, 0, 0];
  state.balance = 0;
  state.skin = 'lcd';
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS) ?? 'null') as Partial<{
      volume: number;
      muted: boolean;
      shuffle: boolean;
      repeat: Repeat;
      eq: number[];
      balance: number;
      skin: Skin;
    }> | null;
    if (s) {
      if (typeof s.volume === 'number') state.volume = Math.min(1, Math.max(0, s.volume));
      state.muted = s.muted === true;
      state.shuffle = s.shuffle === true;
      if (s.repeat === 'all' || s.repeat === 'one') state.repeat = s.repeat;
      if (
        Array.isArray(s.eq) &&
        s.eq.length === BANDS.length &&
        s.eq.every((v) => typeof v === 'number')
      )
        state.eq = s.eq.map((v) => Math.max(-GAIN_MAX, Math.min(GAIN_MAX, v)));
      if (typeof s.balance === 'number') state.balance = Math.max(-1, Math.min(1, s.balance));
      if (SKINS.some(([k]) => k === s.skin)) state.skin = s.skin!;
    }
  } catch {
    /* nothing kept: the defaults */
  }
  if (el) {
    el.volume = state.volume;
    el.muted = state.muted;
  }
  try {
    const kept = JSON.parse(localStorage.getItem(KEY) ?? 'null') as {
      queue?: Track[];
      at?: number;
    } | null;
    if (kept?.queue?.length && typeof kept.at === 'number' && kept.queue[kept.at]) {
      state.queue = kept.queue;
      state.at = kept.at;
      state.length = now()?.duration ?? 0;
      if (state.shuffle) state.order = shuffled(state.at);
    }
  } catch {
    /* nothing kept, or nothing readable: an empty player */
  }
  describe();
}

// ── the system's own controls (desktop media keys, the OS's player) ────────────────────────────
function describe() {
  const session = typeof navigator !== 'undefined' ? navigator.mediaSession : undefined;
  if (!session) return;
  const t = now();
  try {
    session.metadata =
      t && typeof MediaMetadata !== 'undefined'
        ? new MediaMetadata({
            title: t.title,
            artist: t.artist,
            album: t.album,
            artwork: t.cover ? [{ src: assetUrl(t.cover), sizes: '512x512' }] : [],
          })
        : null;
    session.setActionHandler('play', t ? toggle : null);
    session.setActionHandler('pause', t ? toggle : null);
    session.setActionHandler('stop', t ? halt : null);
    session.setActionHandler('previoustrack', t ? previous : null);
    session.setActionHandler('nexttrack', t ? next : null);
    session.setActionHandler('seekbackward', t ? () => skip(-10) : null);
    session.setActionHandler('seekforward', t ? () => skip(10) : null);
    session.setActionHandler(
      'seekto',
      t ? (d) => d.seekTime !== undefined && seek(d.seekTime) : null,
    );
  } catch {
    /* an action this browser does not know: the others still work */
  }
}

// **Save media, on whichever device** (`decisions.md` 2026-11-17). The computer's server runs the tools
// itself (`fm-serve/src/media.rs`); on the phone the shell runs them (yt-dlp inside the app,
// `MediaRun.kt`) and the core says what to ask and files the finished file (`fm_app::media::keep`),
// the same as the computer's. The sheet (`SaveMedia.svelte`) asks only this.
import * as ipc from './ipc';

interface Shell {
  workDir(): string;
  probe(id: string, url: string): void;
  start(id: string, url: string, args: string): void;
  jobs(): string;
  cancel(id: string): void;
  forget(id: string): void;
}
interface Raw {
  id: string;
  state: string;
  progress: number | null;
  title: string;
  file: string;
  info: string;
  probe: string;
  error: string;
}

const shell = () => (globalThis as { __fmMedia?: Shell }).__fmMedia;
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const asks = new Map<string, ipc.MediaAsk>();
const ended = new Map<number, ipc.MediaJob>();
let seq = 0;

function raw(s: Shell): Raw[] {
  try {
    return JSON.parse(s.jobs()) as Raw[];
  } catch {
    return [];
  }
}

export function tools(): Promise<ipc.MediaTools> {
  if (shell())
    return Promise.resolve({
      ytdlp: true,
      ffmpeg: true,
      ytdlp_version: '',
      ffmpeg_version: '',
      supported: true,
    });
  return ipc.mediaTools();
}

export async function probe(url: string): Promise<ipc.MediaProbe> {
  const s = shell();
  if (!s) return ipc.mediaProbe(url);
  const id = `probe-${Date.now()}-${seq++}`;
  s.probe(id, url);
  for (let i = 0; i < 400; i++) {
    await wait(250);
    const j = raw(s).find((x) => x.id === id);
    if (j?.state === 'probed') {
      s.forget(id);
      return ipc.mediaProbeRead(j.probe);
    }
    if (j?.state === 'failed') {
      s.forget(id);
      throw new Error(await ipc.mediaExplain(j.error));
    }
  }
  s.forget(id);
  throw new Error('The media could not be looked at in time.');
}

export async function save(ask: ipc.MediaAsk): Promise<{ id: number }> {
  const s = shell();
  if (!s) return ipc.mediaSave(ask);
  const id = (Date.now() % 1e9) * 100 + (seq++ % 100);
  // The core's plan for the phone: the runs (no ffmpeg in the app) and whether to join them.
  const plan = await ipc.mediaArgs(ask, `${s.workDir()}/${id}`);
  asks.set(String(id), ask);
  s.start(String(id), ask.url, JSON.stringify(plan));
  return { id };
}

export async function jobs(): Promise<ipc.MediaJob[]> {
  const s = shell();
  if (!s) return ipc.mediaJobs();
  const running: ipc.MediaJob[] = [];
  for (const j of raw(s)) {
    if (j.id.startsWith('probe-')) continue;
    const id = Number(j.id);
    const ask = asks.get(j.id);
    const base: ipc.MediaJob = {
      id,
      url: ask?.url ?? '',
      title: j.title,
      what: ask?.what ?? 'audio',
      state: 'downloading',
      progress: j.progress,
      error: null,
      note: null,
    };
    if (j.state === 'fetched' && ask) {
      // Filed once: through the core, the same as the computer's.
      asks.delete(j.id);
      ended.set(id, { ...base, state: 'filing', progress: null });
      let info: unknown = null;
      try {
        info = JSON.parse(j.info || 'null');
      } catch {
        /* no details: the note is named from the file */
      }
      void ipc
        .mediaKeep(ask, j.file, info, `media-${j.id}`)
        .then((r) => ended.set(id, { ...base, state: 'done', progress: 1, note: r.note }))
        .catch((e) => ended.set(id, { ...base, state: 'failed', error: String(e) }))
        .finally(() => s.forget(j.id));
    } else if (j.state === 'failed') {
      ended.set(id, { ...base, state: 'failed', error: await ipc.mediaExplain(j.error) });
      s.forget(j.id);
    } else if (j.state === 'cancelled') {
      ended.set(id, { ...base, state: 'cancelled' });
      s.forget(j.id);
    } else if (!ended.has(id)) {
      running.push({ ...base, state: j.state as ipc.MediaJob['state'] });
    }
  }
  return [...ended.values(), ...running];
}

/** The checked tracks not yet started, for the sheet to show. */
let left = 0;
let halted = false;
let running = false;
export const waitingCount = () => left;
/** Start none of the tracks still waiting (the one downloading goes on, or is cancelled alone). */
export function stopEach(): void {
  halted = true;
  keep([]);
}

/** **The tracks not yet filed, kept on this device** (2026-11-21: an album saved on a phone that
 *  went to the background kept its first track only — the queue lived in the page, which Android
 *  paused and then closed). Each is taken off once it has ended; the next start carries on. */
const QUEUE_KEY = 'fm-media-queue';
/** Which tab carries a queue on (a computer may have several open): its mark, renewed each few
 *  seconds; another tab leaves the queue alone while it is fresh. */
const OWNER_KEY = 'fm-media-queue-owner';
const me = Math.random().toString(36).slice(2);
function keep(list: ipc.MediaAsk[]) {
  try {
    if (list.length) localStorage.setItem(QUEUE_KEY, JSON.stringify(list));
    else localStorage.removeItem(QUEUE_KEY);
  } catch {
    /* a device that keeps nothing carries nothing over */
  }
}
function kept(): ipc.MediaAsk[] {
  try {
    const v = JSON.parse(localStorage.getItem(QUEUE_KEY) ?? '[]');
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}
function claim(): boolean {
  try {
    const o = JSON.parse(localStorage.getItem(OWNER_KEY) ?? 'null') as {
      id: string;
      at: number;
    } | null;
    if (o && o.id !== me && Date.now() - o.at < 15000) return false;
    localStorage.setItem(OWNER_KEY, JSON.stringify({ id: me, at: Date.now() }));
    return true;
  } catch {
    return true;
  }
}
function release() {
  try {
    const o = JSON.parse(localStorage.getItem(OWNER_KEY) ?? 'null') as { id: string } | null;
    if (o?.id === me) localStorage.removeItem(OWNER_KEY);
  } catch {
    /* nothing to give back */
  }
}

/** **Several tracks at once** (2026-11-21, a Bandcamp album): saved one after another, each started
 *  when the one before has ended, so two never add themselves to one playlist together (its update
 *  is refused on a stale version) and the phone runs one download at a time. Kept on the device
 *  until each has ended, so a paused or closed app carries on at its next start (`resumeQueue`); a
 *  track whose song is already in the library (by its page) is not saved twice. `onstart` is told
 *  each job as it starts. */
export async function saveEach(list: ipc.MediaAsk[], onstart: (id: number) => void): Promise<void> {
  if (running) return;
  running = true;
  halted = false;
  let rest = [...list];
  keep(rest);
  left = rest.length;
  const beat = setInterval(claim, 5000);
  claim();
  try {
    let have = new Set<string>();
    try {
      have = new Set(
        (await ipc.musicLibrary())
          .map((n) => n.props.source)
          .filter((x): x is string => typeof x === 'string'),
      );
    } catch {
      /* no library to ask: everything is saved */
    }
    while (rest.length && !halted) {
      const ask = rest[0];
      left = rest.length - 1;
      const page = ask.meta?.source;
      if (!(page && have.has(page))) {
        let id: number | null = null;
        try {
          ({ id } = await save(ask));
        } catch {
          id = null;
        }
        if (id !== null) {
          onstart(id);
          for (let missing = 0; missing < 10;) {
            await wait(1000);
            let j: ipc.MediaJob | undefined;
            try {
              j = (await jobs()).find((x) => x.id === id);
            } catch {
              continue;
            }
            if (!j) missing++;
            else if (['done', 'failed', 'cancelled'].includes(j.state)) break;
          }
        }
      }
      rest = rest.slice(1);
      keep(halted ? [] : rest);
    }
  } finally {
    clearInterval(beat);
    release();
    running = false;
    left = 0;
  }
}

/** **Carry on with a queue left by a paused or closed app** (at start, and on coming back to the
 *  page). Only one tab does; answers how many tracks it carries on with (0: none, or another tab). */
export function resumeQueue(): number {
  if (running) return 0;
  const list = kept();
  if (!list.length || !claim()) return 0;
  void saveEach(list, () => {});
  return list.length;
}

export function cancel(id: number): Promise<unknown> {
  const s = shell();
  if (!s) return ipc.mediaCancel(id);
  s.cancel(String(id));
  return Promise.resolve(null);
}

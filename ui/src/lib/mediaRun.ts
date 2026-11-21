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
export const waitingCount = () => left;
/** Start none of the tracks still waiting (the one downloading goes on, or is cancelled alone). */
export function stopEach(): void {
  halted = true;
}

/** **Several tracks at once** (2026-11-20, a Bandcamp album): saved one after another, each started
 *  when the one before has ended, so two never add themselves to one playlist together (its update
 *  is refused on a stale version) and the phone runs one download at a time. It goes on after the
 *  sheet is closed, while the app is open: on the phone a finished download is filed while the
 *  jobs are read, which this does itself. `onstart` is told each job as it starts. */
export async function saveEach(list: ipc.MediaAsk[], onstart: (id: number) => void): Promise<void> {
  halted = false;
  left = list.length;
  try {
    for (const ask of list) {
      if (halted) break;
      left--;
      let id: number;
      try {
        ({ id } = await save(ask));
      } catch {
        continue;
      }
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
  } finally {
    left = 0;
  }
}

export function cancel(id: number): Promise<unknown> {
  const s = shell();
  if (!s) return ipc.mediaCancel(id);
  s.cancel(String(id));
  return Promise.resolve(null);
}

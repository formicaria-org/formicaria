// **Where a download goes** (owner, 2026-11-15): *"I do not want all the downloads to end in the
// vault, but there should be an option that is on by default every time … only one copy, the user can
// select the location."*
//
// So every download from a web page pauses at once and asks: **Keep in formicaria** (the default, the
// button that has the focus), **Downloads folder**, or **Choose a folder…**. One copy, always: kept in
// formicaria, it is not also left in Downloads. A song (or an album bought as a zip) becomes songs in
// Music, made exactly as *Add a song* makes them (`src/lib/music/songDraft.ts`); any other file
// becomes a note with the file attached, saying where it came from.
import { formOf, label, songDraft } from '../../src/lib/music/songDraft';
import type { ObjectMeta } from '../../src/lib/types';
import { isAudioName, unzip } from './unzip';

declare const browser: {
  runtime: { id: string; getURL(p: string): string };
  storage: { managed: { get(): Promise<{ key?: string; server?: string }> } };
  downloads: {
    pause(id: number): Promise<void>;
    resume(id: number): Promise<void>;
    cancel(id: number): Promise<void>;
    erase(q: { id: number }): Promise<unknown>;
    removeFile(id: number): Promise<void>;
    search(q: { id: number }): Promise<{ state: string }[]>;
    download(d: { url: string; filename?: string; saveAs?: boolean }): Promise<number>;
  };
  windows: {
    create(d: { url: string; type: 'popup'; width: number; height: number }): Promise<unknown>;
  };
  notifications: {
    create(d: {
      type: 'basic';
      title: string;
      message: string;
      iconUrl?: string;
    }): Promise<unknown>;
  };
};

/** What the question window needs, carried in its own address (the background may sleep meanwhile). */
export interface Pending {
  id: number;
  url: string;
  name: string;
  mime: string;
  size: number;
  referrer: string;
}

export type Where = 'vault' | 'downloads' | 'folder';

const today = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};
const host = (u: string) => {
  try {
    return new URL(u).hostname.replace(/^www\./, '');
  } catch {
    return u;
  }
};
const notify = (title: string, message: string) =>
  browser.notifications
    .create({ type: 'basic', title, message, iconUrl: 'icon.svg' })
    .catch(() => {});

/** A new download: pause it and ask where it should go. Downloads this add-on starts itself (the
 *  person's own choice, carried out) and anything not fetched from a web address are left alone. */
export async function onCreated(item: {
  id: number;
  url: string;
  filename: string;
  mime?: string;
  totalBytes?: number;
  referrer?: string;
  byExtensionId?: string;
}) {
  if (item.byExtensionId === browser.runtime.id) return;
  if (!/^https?:/.test(item.url)) return;
  await browser.downloads.pause(item.id).catch(() => {});
  const p: Pending = {
    id: item.id,
    url: item.url,
    name:
      (item.filename || '').split(/[\\/]/).pop() ||
      decodeURIComponent(item.url.split('/').pop() || 'download'),
    mime: item.mime ?? '',
    size: item.totalBytes ?? -1,
    referrer: item.referrer ?? '',
  };
  const q = new URLSearchParams({ p: JSON.stringify(p) });
  await browser.windows.create({
    url: browser.runtime.getURL(`choose.html?${q}`),
    type: 'popup',
    width: 480,
    height: 230,
  });
}

/** Carry out the person's choice. */
export async function choose(where: Where, p: Pending) {
  if (where === 'downloads') {
    await browser.downloads.resume(p.id).catch(() => {});
    return;
  }
  const [was] = await browser.downloads.search({ id: p.id }).catch(() => []);
  if (was?.state === 'complete') await browser.downloads.removeFile(p.id).catch(() => {});
  else await browser.downloads.cancel(p.id).catch(() => {});
  await browser.downloads.erase({ id: p.id }).catch(() => {});
  if (where === 'folder') {
    await browser.downloads
      .download({ url: p.url, filename: p.name, saveAs: true })
      .catch(() => {});
    return;
  }
  try {
    const said = await keepInVault(p);
    notify('Kept in formicaria', said);
  } catch (e) {
    // The one copy must land somewhere: if formicaria could not take it, it goes to Downloads.
    await browser.downloads.download({ url: p.url, filename: p.name }).catch(() => {});
    notify(
      'Saved to Downloads instead',
      `formicaria could not keep ${p.name}: ${e instanceof Error ? e.message : String(e)}`,
    );
  }
}

async function config() {
  const m = await browser.storage.managed
    .get()
    .catch(() => ({}) as { key?: string; server?: string });
  if (!m.key || !m.server) throw new Error('this works inside formicaria’s own browser');
  return { key: m.key, server: m.server.replace(/\/+$/, '') };
}

async function post<T>(path: string, body: BodyInit, json: boolean): Promise<T> {
  const c = await config();
  const r = await fetch(`${c.server}/api/${path}`, {
    method: 'POST',
    headers: {
      'X-Formicaria-Key': c.key,
      'Content-Type': json ? 'application/json' : 'application/octet-stream',
    },
    body,
  });
  const text = await r.text();
  if (!r.ok) throw new Error(text || `formicaria answered ${r.status}`);
  return (text ? JSON.parse(text) : null) as T;
}

const ingest = (name: string, data: Uint8Array) =>
  post<ObjectMeta>(
    `ingest?name=${encodeURIComponent(name)}&vault=`,
    new Blob([data as BlobPart]),
    false,
  );

const createNotes = (notes: unknown[]) =>
  post<ObjectMeta[]>('create_notes', JSON.stringify({ notes, vault: '' }), true);

/** Fetch the file again — with the browser's own cookies, so a bought album's link still works — and
 *  keep it in formicaria. Answers a sentence for the notification. */
export async function keepInVault(p: Pending): Promise<string> {
  const r = await fetch(p.url, { credentials: 'include', referrer: p.referrer || undefined });
  if (!r.ok) throw new Error(`the site answered ${r.status}`);
  const bytes = new Uint8Array(await r.arrayBuffer());
  if (!bytes.length) throw new Error('the site sent nothing');
  const from = p.referrer || p.url;

  if (/\.zip$/i.test(p.name) || p.mime === 'application/zip') {
    const tracks = await unzip(bytes, isAudioName).catch(() => []);
    if (tracks.length) {
      const known = await knownAudio();
      let made = 0;
      for (const t of tracks) if (await song(t.name, t.data, known)) made++;
      return made
        ? `${made} song${made === 1 ? '' : 's'} from ${p.name}, in Music.`
        : `Every song in ${p.name} was already in your music.`;
    }
  }
  if (isAudioName(p.name) || p.mime.startsWith('audio/')) {
    const made = await song(p.name, bytes, await knownAudio());
    return made ? `${p.name}, in Music.` : `${p.name} was already in your music.`;
  }
  const asset = await ingest(p.name, bytes);
  const ref = (asset.assets[0] ?? '').replace(/^sha256:/, 'sha256-');
  await createNotes([
    {
      title: p.name,
      body: `[${label(p.name)}](asset:${ref})\n\n---\n\nDownloaded from [${host(from)}](${from}), kept on ${today()}.\n`,
      tags: ['web', 'shelf:Web'],
      props: { source_url: from, saved: today() },
    },
  ]);
  return `${p.name}, as a note with the file.`;
}

/** The audio every song already holds, so a file already in Music is not added twice. */
async function knownAudio(): Promise<Set<string>> {
  const songs = await post<ObjectMeta[]>('music_library', '{}', true).catch(() => []);
  return new Set(songs.map((s) => s.props.audio).filter((a): a is string => typeof a === 'string'));
}

/** One audio file → its song note, as *Add a song* makes it. False when Music already has it. */
async function song(name: string, data: Uint8Array, known: Set<string>): Promise<boolean> {
  const asset = await ingest(name, data);
  const audio = asset.assets[0] ?? '';
  const mime = typeof asset.props.mime === 'string' ? asset.props.mime : '';
  if (!mime.startsWith('audio/')) return false;
  if (known.has(audio)) return false;
  known.add(audio);
  const form = formOf(asset, name);
  if (!form.title.trim()) form.title = name;
  await createNotes([songDraft(asset, form)]);
  return true;
}

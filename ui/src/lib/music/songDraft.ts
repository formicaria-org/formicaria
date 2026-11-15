// **A song note from an audio file** (`decisions.md` 2026-11-13, *a sixth group, Music*): what *Add a
// song* makes, and — since 0.32.0 — what formicaria's browser makes of a song you downloaded and kept
// in formicaria (`ui/addon/`), so a song is the same note however it arrived. Pure.
import type { NoteDraft, ObjectMeta } from '../types';

/** The fields of a song, as typed: text, even for a number, until it is saved. */
export interface Form {
  title: string;
  artist: string;
  album: string;
  album_artist: string;
  track: string;
  disc: string;
  year: string;
  genre: string;
}

export const TEXT: (keyof Form)[] = ['title', 'artist', 'album', 'album_artist', 'genre'];
export const NUMBER: (keyof Form)[] = ['track', 'disc', 'year'];

/** "03 - Artist - Title.mp3" → its artist and title, for a file with no tags. */
export function fromName(name: string): Pick<Form, 'artist' | 'title'> {
  const bare = name.replace(/\.[a-z0-9]{2,5}$/i, '').replace(/^\s*\d{1,3}\s*[-.)_ ]\s*/, '');
  const parts = bare.split(/\s+-\s+/);
  return parts.length >= 2
    ? { artist: parts[0].trim(), title: parts.slice(1).join(' - ').trim() }
    : { artist: '', title: bare.trim() };
}

/** The fields as the file's own tags give them (read at upload), else guessed from its name. */
export function formOf(asset: ObjectMeta, name: string): Form {
  const p = asset.props;
  const t = (k: string) => (typeof p[k] === 'string' ? (p[k] as string) : '');
  const n = (k: string) => (typeof p[k] === 'number' ? String(p[k]) : '');
  const guess = fromName(name);
  return {
    title: t('tag_title') || guess.title,
    artist: t('artist') || guess.artist,
    album: t('album'),
    album_artist: t('album_artist'),
    track: n('track'),
    disc: n('disc'),
    year: n('year'),
    genre: t('genre'),
  };
}

/** Escape the label half of a Markdown link, as a note's own attachments are. */
export const label = (t: string) => t.replace(/\\/g, '\\\\').replace(/[[\]]/g, '\\$&');

/** The song note for an uploaded audio file (`asset`) with its fields. */
export function songDraft(asset: ObjectMeta, form: Form): NoteDraft {
  const audio = asset.assets[0];
  const title = form.title.trim();
  const props: Record<string, string | number> = { log: 'music', audio };
  for (const k of TEXT) if (k !== 'title' && form[k].trim()) props[k] = form[k].trim();
  for (const k of NUMBER) {
    const n = Number(form[k]);
    if (form[k].trim() && Number.isInteger(n) && n > 0) props[k] = n;
  }
  const length = asset.props.duration_s;
  if (typeof length === 'number') props.duration_s = length;
  const cover = asset.props.cover;
  if (typeof cover === 'string') props.cover = cover;
  return {
    title,
    props,
    body: `![${label(title)}](asset:${audio.replace(/^sha256:/, 'sha256-')})\n`,
  };
}

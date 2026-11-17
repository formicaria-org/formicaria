// **What a page holds that can be kept** (`decisions.md` 2026-11-17, *media from any site*): its own
// video and audio, its embedded players (a news article's YouTube video), and the media files and
// streams it was seen fetching. Read from the page's document (the computer's add-on: the live page;
// the phone: the HTML its shell hands over). The page itself is always offered last: yt-dlp finds
// what is embedded in a page it knows.

export type MediaKind = 'video' | 'audio' | 'player' | 'stream' | 'page';
export interface Candidate {
  url: string;
  kind: MediaKind;
  /** What the person reads: the player's site, the file's name, the page's title. */
  label: string;
}

/** Embedded players by their address: the site's name and the address yt-dlp reads best. */
const PLAYERS: { test: RegExp; site: string; watch?: (m: RegExpMatchArray) => string }[] = [
  {
    test: /^https?:\/\/(?:www\.)?youtube(?:-nocookie)?\.com\/embed\/([\w-]{6,})/,
    site: 'YouTube',
    watch: (m) => `https://www.youtube.com/watch?v=${m[1]}`,
  },
  { test: /^https?:\/\/(?:www\.)?youtube\.com\/watch\?/, site: 'YouTube' },
  { test: /^https?:\/\/youtu\.be\//, site: 'YouTube' },
  {
    test: /^https?:\/\/player\.vimeo\.com\/video\/(\d+)/,
    site: 'Vimeo',
    watch: (m) => `https://vimeo.com/${m[1]}`,
  },
  { test: /^https?:\/\/(?:www\.)?dailymotion\.com\/embed\/video\//, site: 'Dailymotion' },
  { test: /^https?:\/\/geo\.dailymotion\.com\/player/, site: 'Dailymotion' },
  { test: /^https?:\/\/w\.soundcloud\.com\/player\//, site: 'SoundCloud' },
  { test: /^https?:\/\/bandcamp\.com\/EmbeddedPlayer\//, site: 'Bandcamp' },
  { test: /^https?:\/\/(?:www\.)?twitch\.tv\/|^https?:\/\/player\.twitch\.tv\//, site: 'Twitch' },
  { test: /^https?:\/\/(?:www\.)?facebook\.com\/plugins\/video/, site: 'Facebook' },
  { test: /^https?:\/\/(?:www\.)?tiktok\.com\/embed/, site: 'TikTok' },
  { test: /^https?:\/\/(?:www\.)?instagram\.com\/(?:p|reel)\//, site: 'Instagram' },
  {
    test: /^https?:\/\/(?:open\.)?spotify\.com\/embed\/(?:episode|show)\//,
    site: 'Spotify (podcast)',
  },
  { test: /^https?:\/\/embed\.podcasts\.apple\.com\//, site: 'Apple Podcasts' },
  { test: /^https?:\/\/(?:www\.)?arte\.tv\/player\//, site: 'ARTE' },
  { test: /^https?:\/\/(?:www\.)?ted\.com\/talks\//, site: 'TED' },
];

const STREAM = /\.(m3u8|mpd)(\?|$)/i;
const AUDIO = /\.(mp3|m4a|aac|ogg|oga|opus|flac|wav)(\?|$)/i;
const VIDEO = /\.(mp4|m4v|webm|mov|mkv)(\?|$)/i;

function abs(u: string | null | undefined, base: string): string | null {
  if (!u) return null;
  try {
    const a = new URL(u.trim(), base);
    return a.protocol === 'https:' || a.protocol === 'http:' ? a.href : null;
  } catch {
    return null;
  }
}

const fileName = (u: string) => {
  try {
    return decodeURIComponent(new URL(u).pathname.split('/').pop() || u);
  } catch {
    return u;
  }
};

/** A player's site and the address to fetch it from, if the address is one of the known players. */
export function playerOf(url: string): { site: string; url: string } | null {
  for (const p of PLAYERS) {
    const m = url.match(p.test);
    if (m) return { site: p.site, url: p.watch ? p.watch(m) : url };
  }
  return null;
}

/** Everything on the page that can be kept, each once; `seen` are media requests the page made. */
export function findMedia(doc: Document, base: string, seen: string[] = []): Candidate[] {
  const out: Candidate[] = [];
  const have = new Set<string>();
  const add = (c: Candidate) => {
    if (have.has(c.url)) return;
    have.add(c.url);
    out.push(c);
  };
  const title = doc.title?.trim();
  for (const f of Array.from(doc.querySelectorAll('iframe[src], iframe[data-src], embed[src]'))) {
    const u = abs(f.getAttribute('src') || f.getAttribute('data-src'), base);
    const p = u && playerOf(u);
    if (p)
      add({
        url: p.url,
        kind: 'player',
        label: `${p.site} video${f.getAttribute('title') ? `: ${f.getAttribute('title')}` : ''}`,
      });
  }
  // Links to a player (a page's "watch on YouTube").
  for (const a of Array.from(doc.querySelectorAll('a[href]'))) {
    const u = abs(a.getAttribute('href'), base);
    const p = u && playerOf(u);
    if (p && /youtube|vimeo|soundcloud/i.test(p.site) && out.length < 12)
      add({
        url: p.url,
        kind: 'player',
        label: `${p.site}: ${(a.textContent || '').trim().slice(0, 60) || 'linked video'}`,
      });
  }
  for (const el of Array.from(doc.querySelectorAll('video, audio'))) {
    const kind: MediaKind = el.tagName === 'AUDIO' ? 'audio' : 'video';
    const srcs = [
      el.getAttribute('src'),
      ...Array.from(el.querySelectorAll('source')).map((s) => s.getAttribute('src')),
    ];
    for (const s of srcs) {
      const u = abs(s, base);
      if (u && !u.startsWith('blob:'))
        add({
          url: u,
          kind: STREAM.test(u) ? 'stream' : kind,
          label: `${kind === 'audio' ? 'Audio' : 'Video'}: ${fileName(u)}`,
        });
    }
  }
  for (const s of seen) {
    const u = abs(s, base);
    if (!u) continue;
    if (STREAM.test(u)) add({ url: u, kind: 'stream', label: `Stream: ${fileName(u)}` });
    else if (AUDIO.test(u)) add({ url: u, kind: 'audio', label: `Audio: ${fileName(u)}` });
    else if (VIDEO.test(u)) add({ url: u, kind: 'video', label: `Video: ${fileName(u)}` });
  }
  // Open Graph: a page that declares its video or audio.
  for (const k of [
    'og:video',
    'og:video:url',
    'og:video:secure_url',
    'og:audio',
    'og:audio:secure_url',
  ]) {
    const u = abs(doc.querySelector(`meta[property="${k}"]`)?.getAttribute('content'), base);
    if (!u) continue;
    const p = playerOf(u);
    add(
      p
        ? { url: p.url, kind: 'player', label: `${p.site} video` }
        : {
            url: u,
            kind: k.includes('audio') ? 'audio' : 'video',
            label: `${k.includes('audio') ? 'Audio' : 'Video'}: ${fileName(u)}`,
          },
    );
  }
  const page = abs(base, base);
  if (page)
    add({
      url: page,
      kind: 'page',
      label: `The page itself${title ? `: ${title.slice(0, 80)}` : ''}`,
    });
  return out;
}

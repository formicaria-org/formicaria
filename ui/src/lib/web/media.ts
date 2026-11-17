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
const WIKI_AUDIO = /\.(ogg|oga|opus|mp3|wav|flac|mid|midi)$/i;
const WIKI_VIDEO = /\.(webm|ogv|mpg|mpeg)$/i;
const WIKI_FILE =
  /^https?:\/\/(?:[a-z-]+\.)*(?:wikipedia|wikimedia|wiktionary|wikibooks|wikiversity|wikisource|wikivoyage)\.org\/wiki\/File:([^?#]+)$/i;

/** **A Wikimedia file page** (a recording or a video on Wikipedia or Commons): its Commons page, which
 *  yt-dlp reads, and whether it is sound or picture. Wikipedia's player script takes the file's own
 *  address out of the live page and keeps only this (`resource`), so it is the one way to them. */
export function wikimediaFile(
  u: string,
): { url: string; kind: 'audio' | 'video'; name: string } | null {
  const m = u.match(WIKI_FILE);
  if (!m) return null;
  const kind = WIKI_AUDIO.test(m[1]) ? 'audio' : WIKI_VIDEO.test(m[1]) ? 'video' : null;
  if (!kind) return null;
  let name = m[1];
  try {
    name = decodeURIComponent(m[1]);
  } catch {
    /* as written */
  }
  return { url: `https://commons.wikimedia.org/wiki/File:${m[1]}`, kind, name };
}
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
    // A wiki's file page (`/wiki/File:Ant.webm`) is named by its file, without the `File:`.
    return decodeURIComponent(new URL(u).pathname.split('/').pop() || u).replace(/^File:/, '');
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
    // A Wikimedia player: its file page, the same however the player script left the element.
    const wiki = wikimediaFile(abs(el.getAttribute('resource'), base) ?? '');
    if (wiki) {
      add({
        url: wiki.url,
        kind: wiki.kind,
        label: `${wiki.kind === 'audio' ? 'Audio' : 'Video'}: ${wiki.name}`,
      });
      continue;
    }
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
  // A Wikimedia file page (Commons' page of a video or a recording): the media it shows.
  const own = page ? wikimediaFile(page) : null;
  if (own)
    add({
      url: own.url,
      kind: own.kind,
      label: `${own.kind === 'audio' ? 'Audio' : 'Video'}: ${own.name}`,
    });
  // A video site's own page (YouTube's watch page): the video it shows, said as such — its picture
  // plays from a stream the page assembles, so no file address is there to list.
  const site = page ? playerOf(page) : null;
  if (page && site)
    add({
      url: site.url,
      kind: 'player',
      label: `${site.site} video${title ? `: ${title.replace(/ - YouTube$/, '').slice(0, 80)}` : ''}`,
    });
  else if (page)
    add({
      url: page,
      kind: 'page',
      label: `The page itself${title ? `: ${title.slice(0, 80)}` : ''}`,
    });
  return out;
}

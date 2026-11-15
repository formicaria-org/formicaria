// **A page kept from formicaria's own browser, as a note** — `decisions.md` 2026-11-15, *formicaria
// gets its own browser*; the plan is `docs/context/browser-plan-2026-11-15.md`.
//
// Pure: no store, no network, no DOM. The browser add-on (`ui/addon/`) reads the page — its address,
// title, the structured data it publishes for machines (JSON-LD, `citation_*` and Open Graph meta),
// the article text Readability finds, and what the person selected — and this module decides what
// the page *is* and builds the note. The same module serves the add-on on a computer and, in R2, the
// phone's browser, so one page becomes one note shape everywhere.
//
// Read what a page publishes for machines first, then its article text; **guess nothing from its
// look** (the rule `web_recipe.rs` set for recipes, 2026-10-18).

import { recipeBody, type RecipeSource } from '../food/recipeText';
import { RECIPE, recipeWord } from '../templates';
import type { WebRecipe } from '../types';

/** What the add-on read off the page. Every field the page's own. */
export interface PageFacts {
  url: string;
  title: string;
  /** Every JSON-LD block, parsed (unparseable ones dropped). */
  jsonld: unknown[];
  /** `<meta name|property=… content=…>`, every value kept (a paper has many `citation_author`). */
  meta: Record<string, string[]>;
  /** Readability's reading of the page, with its content as Markdown. */
  article?: {
    title?: string;
    byline?: string;
    siteName?: string;
    published?: string;
    excerpt?: string;
    markdown?: string;
  };
  /** What the person had selected when they pressed Save. */
  selection?: string;
}

export type ClipKind = 'recipe' | 'paper' | 'video' | 'song' | 'article';

/** What `clip_save` writes: the note, and for a paper the citation `create_paper` reads. */
export interface ClipDraft {
  kind: ClipKind;
  title: string;
  body: string;
  tags: string[];
  props: Record<string, string | number>;
  bibtex?: string;
}

const first = (m: PageFacts['meta'], ...keys: string[]): string | undefined => {
  for (const k of keys) {
    const v = m[k.toLowerCase()]?.find((x) => x.trim());
    if (v) return v.trim();
  }
  return undefined;
};

/** Every node in the JSON-LD, `@graph`s and arrays flattened. */
export function jsonldNodes(blocks: unknown[]): Record<string, unknown>[] {
  const out: Record<string, unknown>[] = [];
  const walk = (v: unknown) => {
    if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === 'object') {
      const o = v as Record<string, unknown>;
      out.push(o);
      if (o['@graph']) walk(o['@graph']);
    }
  };
  blocks.forEach(walk);
  return out;
}

const typeIs = (node: Record<string, unknown>, ...names: string[]) => {
  const t = node['@type'];
  const list = Array.isArray(t) ? t : [t];
  return list.some((x) => typeof x === 'string' && names.includes(x));
};

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

/** What the page is, from what it publishes for machines; an article when it says nothing. */
export function kindOf(f: PageFacts): ClipKind {
  const nodes = jsonldNodes(f.jsonld);
  if (nodes.some((n) => typeIs(n, 'Recipe'))) return 'recipe';
  if (first(f.meta, 'citation_title')) return 'paper';
  const og = (first(f.meta, 'og:type') ?? '').toLowerCase();
  if (og.startsWith('music.') || og === 'song' || og === 'album') return 'song';
  if (nodes.some((n) => typeIs(n, 'MusicRecording', 'MusicAlbum'))) return 'song';
  if (og.startsWith('video')) return 'video';
  if (nodes.some((n) => typeIs(n, 'VideoObject'))) return 'video';
  const host = hostOf(f.url);
  if (/(^|\.)youtube\.com$/.test(host) && /\/(watch|shorts)/.test(f.url)) return 'video';
  if (host === 'youtu.be' || host === 'vimeo.com') return 'video';
  if (/(^|\.)bandcamp\.com$/.test(host) && /\/(track|album)\//.test(f.url)) return 'song';
  return 'article';
}

/** A link that reopens the page with `text` highlighted — the Text Fragments standard, which
 *  modern browsers follow. Long passages are anchored by their first and last words. */
export function textFragment(url: string, text: string): string {
  const base = url.split('#')[0];
  const words = text.replace(/\s+/g, ' ').trim().split(' ').filter(Boolean);
  if (!words.length) return base;
  const enc = (s: string) => encodeURIComponent(s).replace(/-/g, '%2D');
  const spec =
    words.length <= 10
      ? enc(words.join(' '))
      : `${enc(words.slice(0, 5).join(' '))},${enc(words.slice(-5).join(' '))}`;
  return `${base}#:~:text=${spec}`;
}

/** The selection as a quote with its link back, or nothing. */
function quoted(f: PageFacts): string {
  const sel = f.selection?.trim();
  if (!sel) return '';
  const lines = sel.split(/\n+/).map((l) => `> ${l.trim()}`);
  return `${lines.join('\n>\n')}\n\n[↗ the passage on the page](${textFragment(f.url, sel)})\n\n`;
}

/** The credit line every kept page ends with. */
function credit(f: PageFacts, site: string, today: string): string {
  return `---\n\nFrom [${site}](${f.url}), kept on ${today}.`;
}

const esc = (s: string) => s.replace(/[{}\\]/g, '');

/** The year a citation date gives: `2023/05/04`, `2023-05-04`, `2023`. */
const yearOf = (d?: string) => d?.match(/\b(1[5-9]\d\d|20\d\d|21\d\d)\b/)?.[1];

/** A BibTeX entry from the page's `citation_*` tags (the Highwire tags Google Scholar asks
 *  publishers for), so `create_paper` makes the same note a pasted citation does. */
export function citationBibtex(f: PageFacts): string | undefined {
  const m = f.meta;
  const title = first(m, 'citation_title');
  if (!title) return undefined;
  const authors = (m['citation_author'] ?? []).map((a) => a.trim()).filter(Boolean);
  const year = yearOf(
    first(m, 'citation_publication_date', 'citation_date', 'citation_online_date', 'citation_year'),
  );
  const journal = first(m, 'citation_journal_title');
  const conference = first(m, 'citation_conference_title');
  const doi = first(m, 'citation_doi')?.replace(/^doi:/i, '');
  const firstPage = first(m, 'citation_firstpage');
  const lastPage = first(m, 'citation_lastpage');
  const surname = (authors[0] ?? 'paper').split(/[ ,]/).filter(Boolean)[0] ?? 'paper';
  const key = `${surname.toLowerCase().replace(/[^a-z0-9]/g, '') || 'paper'}${year ?? ''}`;
  const fields: [string, string | undefined][] = [
    ['title', title],
    ['author', authors.length ? authors.join(' and ') : undefined],
    ['year', year],
    [conference ? 'booktitle' : 'journal', conference ?? journal],
    ['volume', first(m, 'citation_volume')],
    ['number', first(m, 'citation_issue')],
    ['pages', firstPage ? (lastPage ? `${firstPage}--${lastPage}` : firstPage) : undefined],
    ['publisher', first(m, 'citation_publisher', 'dc.publisher')],
    ['doi', doi],
    ['url', first(m, 'citation_abstract_html_url') ?? f.url],
  ];
  const body = fields
    .filter(([, v]) => v)
    .map(([k, v]) => `  ${k} = {${esc(v as string)}}`)
    .join(',\n');
  return `@${conference ? 'inproceedings' : 'article'}{${key},\n${body}\n}`;
}

/** Build the note for a page. `recipe` is what `recipe_from_page` read, when the page is one. */
export function buildClip(f: PageFacts, today: string, recipe?: WebRecipe | null): ClipDraft {
  const kind = recipe ? 'recipe' : kindOf(f);
  // The page's own name for its site, else its address — not Readability's guess, which on many
  // pages is the publisher's company.
  const site = first(f.meta, 'og:site_name', 'citation_journal_title') ?? hostOf(f.url);
  const title = (
    (kind === 'paper' && first(f.meta, 'citation_title')) ||
    recipe?.title ||
    first(f.meta, 'og:title') ||
    f.article?.title ||
    f.title ||
    hostOf(f.url)
  ).trim();
  const props: Record<string, string | number> = { source_url: f.url, saved: today };
  if (site) props.site = site;
  const author = f.article?.byline?.trim();
  const published =
    first(f.meta, 'article:published_time', 'citation_publication_date', 'citation_date') ??
    f.article?.published;
  const description =
    first(f.meta, 'citation_abstract', 'og:description', 'description') ?? f.article?.excerpt;

  if (kind === 'recipe' && recipe) {
    const src: RecipeSource = {
      ingredients: recipe.ingredients,
      steps: recipe.steps,
      prepMinutes: recipe.prep_minutes,
      cookMinutes: recipe.cook_minutes,
      totalMinutes: recipe.total_minutes,
      yieldText: recipe.yield_text,
      url: recipe.url || f.url,
      credit: [
        recipe.site,
        recipe.author && recipe.author !== recipe.site ? `by ${recipe.author}` : '',
      ]
        .filter(Boolean)
        .join(', '),
      image: recipe.image,
      notes: f.selection?.trim() ? [quoted(f).trim()] : undefined,
    };
    return {
      kind,
      title,
      body: recipeBody(src),
      tags: [RECIPE, 'web'],
      props: {
        ...props,
        recipe: recipeWord(),
        servings: Math.max(1, Math.floor(recipe.servings ?? 1)),
        from: recipe.url || f.url,
      },
    };
  }

  // A byline is an article's; a paper's authors come from its citation, a video's from nowhere.
  if (author && kind === 'article') props.author = author;
  if (published && kind !== 'paper') props.published = published.slice(0, 10);

  if (kind === 'paper') {
    const body = `${quoted(f)}${description ? `${description.trim()}\n\n` : ''}${credit(f, site, today)}`;
    return { kind, title, body, tags: ['web'], props, bibtex: citationBibtex(f) };
  }

  if (kind === 'video' || kind === 'song') {
    const verb = kind === 'video' ? 'Watch' : 'Listen';
    // **Only the item's own structured description.** A video site's loose meta tags can describe
    // another video than the one playing (seen on a real page, 2026-11-15); a wrong text in a
    // person's note is worse than none.
    const item = jsonldNodes(f.jsonld).find((n) =>
      typeIs(n, 'VideoObject', 'MusicRecording', 'MusicAlbum'),
    );
    const own = typeof item?.description === 'string' ? item.description.trim() : '';
    const body =
      `${quoted(f)}[${verb} on ${site}](${f.url})\n\n` +
      `${own ? `${own}\n\n` : ''}${credit(f, site, today)}`;
    return { kind, title, body, tags: ['web', kind, 'shelf:Web'], props };
  }

  const text = f.article?.markdown?.trim() || description?.trim() || '';
  const body = `${quoted(f)}${text ? `${text}\n\n` : ''}${credit(f, site, today)}`;
  return { kind, title, body, tags: ['web', 'shelf:Web'], props };
}

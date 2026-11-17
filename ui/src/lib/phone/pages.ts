// **The phone's web pages, as the app sees them** (`decisions.md` 2026-11-17, *structure*): what the
// Android shell says of its page views (titles, addresses, what was blocked, Back from a first page,
// a link opened as a page), and what the app asks of them (save the page, save its media, open an
// attachment). Moved out of `App.svelte`, which keeps the tabs; this talks to it only through
// [`PageHost`], so it can be read, changed and tested on its own.
import * as ipc from '../ipc';
import type { Pane } from '../panes';
import { readPage, parsePage } from '../web/read';
import { inlinePage } from '../web/inline';
import { buildClip } from '../web/clip';
import { findMedia, type Candidate } from '../web/media';

/** What a page view's protections did (the shell counts them; the Privacy sheet shows them). */
export interface Counts {
  blocked?: number;
  cleaned?: number;
  upgraded?: number;
  banners?: number;
}

/** What the module needs of the app: its tabs and how to move between them, and where to speak. */
export interface PageHost {
  panes(): Pane[];
  front(): Pane | undefined;
  open(over: Partial<Pane>): void;
  close(id: string): void;
  select(index: number): void;
  /** The tab in front before this one. */
  previous(): string | null;
  change(id: string, patch: Partial<Pane>): void;
  counts(id: string, c: Counts): void;
  mediaSheet(s: { page: string; c: Candidate[] }): void;
  newTab(): void;
  today(): string;
  clock(): string;
  say(text: string): void;
  fail(text: string): void;
}

interface Shell {
  html?(id: string): void;
}
const shell = () => (globalThis as { __fmPage?: Shell }).__fmPage;

interface Html {
  id: string;
  url: string;
  title?: string;
  html: string;
  seen?: string[];
}

/** What the next page HTML the shell hands over is for. */
let wanted: { id: string; for: 'save' | 'media' } | null = null;

function ask(host: PageHost, why: 'save' | 'media') {
  const p = host.front();
  if (!p || p.kind !== 'page' || !shell()?.html) return;
  wanted = { id: p.id, for: why };
  shell()!.html!(p.id);
}

/** **Save this page** (≡): read as the computer's *Save page* reads it, kept as one file. */
export const savePage = (host: PageHost) => ask(host, 'save');
/** **Save media** (≡): the page's media, from its HTML and what it fetched. */
export const saveMedia = (host: PageHost) => ask(host, 'media');

async function pageHtml(host: PageHost, d: Html) {
  if (!wanted || d.id !== wanted.id) return;
  const why = wanted.for;
  wanted = null;
  if (why === 'media') {
    host.mediaSheet({ page: d.url, c: findMedia(parsePage(d.html, d.url), d.url, d.seen ?? []) });
    return;
  }
  try {
    const facts = readPage(parsePage(d.html, d.url), d.url);
    // One self-contained file: the page's pictures, styles and fonts put inside the copy.
    try {
      facts.snapshot = (
        await inlinePage(facts.snapshot, d.url, (u) => ipc.webResource(u, d.url))
      ).html;
    } catch {
      /* the plain copy is still kept */
    }
    const draft = buildClip(facts, host.today(), null, host.clock());
    await ipc.clipSave({
      url: facts.url,
      title: draft.title,
      body: draft.body,
      tags: draft.tags,
      props: draft.props,
      bibtex: draft.bibtex ?? '',
      snapshot: facts.snapshot,
    });
    host.say(`Saved “${draft.title}” in your notes.`);
  } catch (e) {
    host.fail(`The page could not be saved: ${String(e)}`);
  }
}

/** **An attachment, on the phone** (the owner on 0.50: *"the saved webpage does not load"*): a saved
 *  page opens in a page tab of its own; anything else goes to Android's *Open with*. */
export async function openAttachment(host: PageHost, ref: string, href: string, label: string) {
  const reference = ref.replace(/^asset:sha256-/i, 'sha256:');
  try {
    const head = await fetch(href, { headers: { Range: 'bytes=0-1023' } })
      .then((r) => r.text())
      .catch(() => '');
    if (/<!doctype html|<html/i.test(head)) {
      const [path] = await ipc.assetPaths([reference]);
      if (path) {
        host.open({
          kind: 'page',
          url: `fmsaved:${path}`,
          pageTitle: label.trim() || 'Saved page',
        });
        return;
      }
    }
    await ipc.openWith(reference);
  } catch (err) {
    host.fail(`It could not be opened: ${String(err)}`);
  }
}

/** **Listen to the page views** while the app is up; answers the function that stops listening. */
export function listen(host: PageHost): () => void {
  // What a page says of itself (its title, where it went): the tab is named by it, and comes back
  // at that address. A saved copy stays the saved copy.
  const onPage = (e: Event) => {
    const d = (e as CustomEvent<{ id: string; title?: string; url?: string }>).detail;
    const p = d && host.panes().find((x) => x.id === d.id && x.kind === 'page');
    if (!p) return;
    const patch: Partial<Pane> = {};
    if (d.title && d.title !== p.pageTitle) patch.pageTitle = d.title;
    if (d.url && d.url !== p.url && !p.url?.startsWith('fmsaved:')) patch.url = d.url;
    if (Object.keys(patch).length) host.change(p.id, patch);
  };
  const onHtml = (e: Event) => void pageHtml(host, (e as CustomEvent<Html>).detail);
  const onPrivacy = (e: Event) => {
    const d = (e as CustomEvent<{ id: string } & Counts>).detail;
    if (d?.id) host.counts(d.id, d);
  };
  // A link in a page, long-pressed and opened as a page: a tab of its own, which knows its opener.
  const onOpen = (e: Event) => {
    const url = (e as CustomEvent<{ url?: string }>).detail?.url ?? '';
    const from = host.front();
    if (/^https?:\/\//.test(url))
      host.open({
        kind: 'page',
        url,
        pageTitle: null,
        openedFrom: from?.kind === 'page' ? from.id : null,
      });
  };
  // **Back with nowhere further back in a page** (the owner on 0.50: links opened from a page were
  // "unclosable"): a page opened from another closes and that one is in front again, as a browser
  // does; otherwise the tab before it; the only tab offers the new tab's choices.
  const onBack = (e: Event) => {
    const id = (e as CustomEvent<{ id: string }>).detail?.id;
    const panes = host.panes();
    const p = panes.find((x) => x.id === id);
    if (!p) return;
    if (p.openedFrom && panes.some((x) => x.id === p.openedFrom)) {
      host.close(p.id);
      const at = host.panes().findIndex((x) => x.id === p.openedFrom);
      if (at !== -1) host.select(at);
      return;
    }
    const prevId = host.previous();
    const prev = prevId ? panes.findIndex((x) => x.id === prevId) : -1;
    if (prev !== -1 && prevId !== p.id) return host.select(prev);
    const other = panes.findIndex((x) => x.id !== p.id);
    if (other !== -1) return host.select(other);
    host.newTab();
  };
  const events: [string, (e: Event) => void][] = [
    ['fm-page', onPage],
    ['fm-page-html', onHtml],
    ['fm-page-privacy', onPrivacy],
    ['fm-page-open', onOpen],
    ['fm-page-back', onBack],
  ];
  for (const [n, f] of events) window.addEventListener(n, f);
  return () => {
    for (const [n, f] of events) window.removeEventListener(n, f);
  };
}

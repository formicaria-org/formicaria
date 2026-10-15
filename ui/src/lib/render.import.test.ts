import { describe, expect, it, vi } from 'vitest';
import { renderInto, type ResolvedAsset } from './render';
import { assetUrl } from './ipc';

// **What an imported Logseq note must look like once it renders.** The Rust side
// (`crates/fm-app/tests/import_corpus.rs`) measures what the importer *writes*; this measures that
// what it writes is something the read view can *show*. The bodies below are the forms the
// importer emits, or will once the plan's fixes land (`docs/context/logseq-import-plan-2026-10-14.md`,
// F2–F7) — so each one is a contract the importer is held to from the other side.

vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: vi.fn() } }));
vi.mock('katex', () => ({
  default: { renderToString: (tex: string) => `<span class="katex">${tex}</span>` },
}));
vi.mock('katex/dist/katex.min.css', () => ({}));

const HASH = 'a'.repeat(64);

/** Resolves every asset to the URL the real read view uses (`assetUrl`, which keeps a `#page=`
 *  fragment a fragment), typed PDF when it carries a page and an image otherwise. */
const resolve = async (src: string): Promise<ResolvedAsset> => {
  const mime = src.includes('#page=') ? 'application/pdf' : 'image/png';
  return { url: assetUrl(src), mime };
};

async function render(body: string): Promise<HTMLElement> {
  const el = document.createElement('div');
  await renderInto(el, body, resolve);
  return el;
}

/** Text a reader would see that is still Logseq syntax rather than its rendering. */
function debris(el: HTMLElement): string[] {
  const text = el.textContent ?? '';
  return ['[[', ']]', '{{', '}}', '](', '{:height', '../assets/', '((', '%3A'].filter((p) =>
    text.includes(p),
  );
}

describe('an imported Logseq note renders', () => {
  it('an image is an image, whatever its file was called', async () => {
    const el = await render(
      `- ![pic one](asset:sha256-${HASH})\n- ![shot(1).png](asset:sha256-${HASH})`,
    );
    expect(el.querySelectorAll('img')).toHaveLength(2);
    expect(debris(el)).toEqual([]);
  });

  it('a linked file is a link to its blob, opened beside the note', async () => {
    const el = await render(`- A slide deck: [deck.pdf](asset:sha256-${HASH})`);
    const a = el.querySelector('a.asset-link');
    expect(a?.getAttribute('href')).toContain('/api/blob/');
    expect(a?.getAttribute('target')).toBe('_blank');
    expect(debris(el)).toEqual([]);
  });

  it('a label holding brackets survives as text', async () => {
    const el = await render(`- [\\[Draft\\]_notes.pdf](asset:sha256-${HASH})`);
    expect(el.querySelector('a.asset-link')?.textContent).toBe('[Draft]_notes.pdf');
  });

  it('a PDF highlight links to its page of the PDF', async () => {
    const el = await render(`- a highlighted sentence [p. 2](asset:sha256-${HASH}#page=2)`);
    expect(el.querySelector('a.asset-link')?.getAttribute('href')).toMatch(
      /^\/api\/blob\/[^#]+#page=2$/,
    );
  });

  it('a video is a link a reader can follow', async () => {
    const el = await render(
      `- A video: [YouTube video](https://www.youtube.com/watch?v=dQw4w9WgXcQ)`,
    );
    expect(el.querySelector('a')?.getAttribute('href')).toContain('youtube.com');
    expect(debris(el)).toEqual([]);
  });

  it('a page link is a link, and display math is math', async () => {
    const el = await render(`- see [Beta: Colon](note:01KZ0000000000000000000000)\n- $$E = mc^2$$`);
    expect(el.querySelector('a')?.getAttribute('href')).toBe('note:01KZ0000000000000000000000');
    expect(el.querySelector('.katex')).not.toBeNull();
    expect(debris(el)).toEqual([]);
  });

  it('what Logseq leaves today is visible debris — the harness must see it', async () => {
    // The importer's current output for a sized image and a macro. If this ever stops reading as
    // debris, the `debris` detector has gone blind, not the importer better.
    const el = await render(
      `- ![b](asset:sha256-${HASH}){:height 100, :width 200}\n- {{video https://x.y/z}}`,
    );
    expect(debris(el)).toEqual(expect.arrayContaining(['{{', '{:height']));
  });
});

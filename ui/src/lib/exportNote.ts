// **⋯ → Export** (`decisions.md` 2026-11-22, later): a note as a file the person keeps or sends on.
// Nothing leaves the device here: each is a download, or the browser's own print (to PDF).
import { noteFile } from './ipc';

const safeName = (title: string | null | undefined) =>
  (title || 'note')
    .replace(/[\\/:*?"<>|]+/g, ' ')
    .trim()
    .slice(0, 80) || 'note';

function download(name: string, type: string, data: string | Blob) {
  const blob = typeof data === 'string' ? new Blob([data], { type }) : data;
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** The note's own Markdown file, as it is kept. */
export async function exportMarkdown(id: string): Promise<void> {
  const f = await noteFile(id);
  download(`${safeName(f.title)}.md`, 'text/markdown', f.text);
}

/** One HTML page of the note as it reads, its pictures included (made into data inside the page). */
export async function exportHtml(title: string, read: HTMLElement): Promise<void> {
  const copy = read.cloneNode(true) as HTMLElement;
  for (const img of Array.from(copy.querySelectorAll('img'))) {
    try {
      const res = await fetch(img.src);
      const blob = await res.blob();
      img.src = await new Promise<string>((done) => {
        const r = new FileReader();
        r.onload = () => done(String(r.result));
        r.readAsDataURL(blob);
      });
    } catch {
      /* a picture that cannot be read stays a link */
    }
  }
  copy
    .querySelectorAll('button, [contenteditable]')
    .forEach((e) => e.removeAttribute('contenteditable'));
  const esc = (t: string) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;');
  const page = `<!doctype html><html><head><meta charset="utf-8"><title>${esc(title)}</title>
<style>body{font:16px/1.6 system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 1rem;color:#1d1d22}
img{max-width:100%}pre{background:#f4f2ee;padding:.8rem;overflow:auto}table{border-collapse:collapse}
td,th{border:1px solid #ddd;padding:.2rem .5rem}</style></head><body><h1>${esc(title)}</h1>${copy.innerHTML}</body></html>`;
  download(`${safeName(title)}.html`, 'text/html', page);
}

/** The browser's print, which offers *Save as PDF*: the note page prints alone (`@media print`). */
export function exportPdf(): void {
  window.print();
}

// The question every download asks (`downloads.ts`): where should it go? formicaria is the default —
// the button with the focus, so Enter keeps it there.
import type { Pending, Where } from './downloads';

declare const browser: { runtime: { sendMessage(m: unknown): Promise<unknown> } };

const p = JSON.parse(new URLSearchParams(location.search).get('p') ?? '{}') as Pending;
const $ = (id: string) => document.getElementById(id) as HTMLElement;
const mb = (n: number) =>
  n < 0
    ? ''
    : n < 1024 * 1024
      ? `${Math.max(1, Math.round(n / 1024))} KB`
      : `${(n / 1048576).toFixed(1)} MB`;
const host = (u: string) => {
  try {
    return new URL(u).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
};

$('name').textContent = p.name;
$('what').textContent = [
  mb(p.size),
  host(p.referrer || p.url) && `from ${host(p.referrer || p.url)}`,
]
  .filter(Boolean)
  .join(' · ');
if (/\.zip$/i.test(p.name))
  ($('vault') as HTMLButtonElement).textContent = 'Keep in formicaria (songs go to Music)';

let done = false;
const go = (where: Where) => {
  if (done) return;
  done = true;
  void browser.runtime
    .sendMessage({ type: 'download-choice', where, p })
    .finally(() => window.close());
};
$('vault').onclick = () => go('vault');
$('downloads').onclick = () => go('downloads');
$('folder').onclick = () => go('folder');
($('vault') as HTMLButtonElement).focus();
// Closing the window without choosing is choosing the usual place: the download is never lost.
window.addEventListener('beforeunload', () => {
  if (!done) void browser.runtime.sendMessage({ type: 'download-choice', where: 'downloads', p });
});

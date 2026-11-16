// **Mail is your webmail** (`decisions.md` 2026-11-17): Gmail unless the person sets another address.
// On the phone it opens as a page inside formicaria (`PageItem`); elsewhere in a browser tab. In
// formicaria's browser on a computer the ant's panel keeps its own copy of the address (the add-on's
// storage). The read-only copy of chosen Gmail labels stays, for search and the assistant.
const KEY = 'fm-webmail';
export const DEFAULT_WEBMAIL = 'https://mail.google.com/';

export function webmail(): string {
  try {
    const v = localStorage.getItem(KEY);
    if (v && /^https:\/\/\S+$/.test(v)) return v;
  } catch {
    /* no storage: the default */
  }
  return DEFAULT_WEBMAIL;
}

/** Keep `url` as the webmail, if it is an https address; answers whether it was kept. */
export function setWebmail(url: string): boolean {
  const v = url.trim();
  if (!/^https:\/\/\S+$/.test(v)) return false;
  try {
    localStorage.setItem(KEY, v);
  } catch {
    return false;
  }
  return true;
}

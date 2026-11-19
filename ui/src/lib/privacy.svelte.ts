// **This device's privacy switches** (`decisions.md` 2026-11-17; `crates/fm-app/src/privacy.rs`):
// read once, changed through the server (which keeps them per device), and handed to the phone's
// page views (`__fmPage.privacy`), which apply them. The computer's browser reads them itself
// (`formicaria.cfg`).
import * as ipc from './ipc';

const DEFAULT_ENGINE = 'https://duckduckgo.com/?q=%s';

const state = $state({ p: null as ipc.Privacy | null, error: '' });
let asked: Promise<void> | null = null;

interface PageShell {
  privacy?(json: string): void;
  pageScript?(name: string, text: string): void;
  vpn?(): boolean;
}
/** The shell that draws web pages on the phone, when there is one. */
function shell(): PageShell | undefined {
  return (globalThis as { __fmPage?: PageShell }).__fmPage;
}
let scriptGiven = false;
function tell(p: ipc.Privacy) {
  const s = shell();
  if (!s) return;
  try {
    s.privacy?.(JSON.stringify(p));
  } catch {
    /* an older shell: it keeps its own defaults */
  }
  // The scripts the pages run (`ui/page/`, built into `dist/page/`), once: the cookie pop-ups, and
  // the lists' procedural hide rules, and the fingerprinting defence.
  if (!scriptGiven && s.pageScript) {
    scriptGiven = true;
    for (const name of ['consent', 'procedural', 'fingerprint'])
      void fetch(`/page/${name}.js`)
        .then((r) => (r.ok ? r.text() : Promise.reject(new Error(String(r.status)))))
        .then((t) => s.pageScript?.(name, t))
        .catch(() => {
          scriptGiven = false;
        });
  }
}

/** The phone build can block trackers in other apps (its shell has the VPN: the F-Droid build). */
export function phoneVpn(): boolean {
  try {
    return shell()?.vpn?.() === true;
  } catch {
    return false;
  }
}

/** This device's switches; `null` until read (asked on first use). */
export function privacy(): ipc.Privacy | null {
  if (!state.p && !asked) void loadPrivacy();
  return state.p;
}
export function privacyError(): string {
  return state.error;
}
export function loadPrivacy(): Promise<void> {
  asked = ipc
    .privacy()
    .then((p) => {
      state.p = p;
      state.error = '';
      tell(p);
    })
    .catch((e) => {
      state.error = String(e);
    });
  return asked;
}
export async function changePrivacy(change: ipc.PrivacyChange): Promise<void> {
  try {
    const p = await ipc.setPrivacy(change);
    // On a computer, ads are blocked by uBlock Origin in formicaria's browser: its switch follows.
    if (typeof change.ads === 'boolean' && !shell())
      await ipc.browserSet({ block_ads: change.ads }).catch(() => {});
    state.p = p;
    state.error = '';
    tell(p);
  } catch (e) {
    state.error = String(e);
  }
}

/** The address that searches the chosen engine for `words`. */
export function searchUrl(words: string): string {
  const p = privacy();
  const tpl = p?.engines.find((e) => e.key === p.search)?.url ?? DEFAULT_ENGINE;
  return tpl.replace('%s', encodeURIComponent(words));
}
/** The chosen engine's name, for a field's label. */
export function searchName(): string {
  const p = privacy();
  return p?.engines.find((e) => e.key === p.search)?.name ?? 'DuckDuckGo';
}

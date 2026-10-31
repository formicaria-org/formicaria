// Updating the app: one press, offered where people are (`decisions.md#toolchain`, 2026-10-29,
// *an update is one press, and the app offers it*).
//
// The server looks for a newer version once a day by itself, but what it found was shown only in
// Settings → About & updates, where nobody goes, and taking it was three presses: *Get*, then
// *Restart into…*, then *Yes, restart*. Now the main window says so in a line of its own
// (`UpdateBanner.svelte`), and **Update** does all of it: download, verify, save what is being
// typed, restart (on Android: open the newest APK in the browser, and Android's installer takes it
// from there — `decisions.md#track-m`, 2026-10-31).
// Settings renders the same state from here, so the two places can never disagree.
//
// Still never silent: nothing is downloaded or installed until somebody presses Update.
import {
  downloadOnPhone,
  phoneUpdatesInBrowser,
  reloadPage,
  updateApply,
  updateCancel,
  updateStart,
  updateStatus,
  type UpdateStatus,
} from './ipc';

/** *Later* hides the offer of that version on this device for this long, then it comes back. A
 *  dismissal that lasts for ever is how a copy stays three versions behind without anyone deciding
 *  that it should. */
const LATER_MS = 3 * 24 * 60 * 60 * 1000;
const LATER_KEY = 'fm-update-later';

function storedLater(): { version: string; at: number } | null {
  try {
    const v = JSON.parse(localStorage.getItem(LATER_KEY) ?? 'null');
    return v && typeof v.version === 'string' && typeof v.at === 'number' ? v : null;
  } catch {
    return null;
  }
}

const state = $state({
  status: null as UpdateStatus | null,
  /// The person pressed Update: carry on into the install once the download is ready.
  going: false,
  restarting: false,
  error: null as string | null,
  phoneMessage: null as string | null,
  later: storedLater(),
});

/** The update picture as the server last described it. `null`: a build with no updater, a paired
 *  device (the host's program is not theirs to replace), or a status that never answered. */
export const status = () => state.status;
export const restarting = () => state.restarting;
/** Update was pressed and has not finished: the line shows progress, not the offer. */
export const going = () => state.going;
export const error = () => state.error;
export const phoneMessage = () => state.phoneMessage;
/** For the Settings controls that get a fresh status of their own (*Check now*, the switch). */
export const setStatus = (u: UpdateStatus) => void (state.status = u);

export function inFlight(p: UpdateStatus['progress']): boolean {
  return !!p && p.stage !== 'ready' && p.stage !== 'failed';
}

/** What each stage is doing, in words. Not a combined percentage: the steps are not comparable in
 *  size, and inventing one figure across them would be making something up. */
export const STAGE_WORDS: Record<string, string> = {
  manifest: 'checking what it contains',
  download: 'downloading',
  unpack: 'checking it runs on this computer',
};

/** Read what the server knows. **Local only**: the server answers from what it last found, so
 *  asking never reaches the network. A failure keeps the last answer — during a restart nothing
 *  answers, and that is not news. */
export async function refresh(): Promise<UpdateStatus | null> {
  try {
    state.status = await updateStatus();
    return state.status;
  } catch {
    return null;
  }
}

/** Should the main window offer it? Only where this copy can install it — a copy that can only be
 *  told (a protected folder, a source run) keeps that in Settings rather than nagging daily — and
 *  not while *Later* holds for this version. Once Update is pressed the line stays until it is done. */
export function offered(): boolean {
  const s = state.status;
  if (!s?.available || !s.can_install) return false;
  if (state.going || state.restarting || state.error || state.phoneMessage) return true;
  if (inFlight(s.progress)) return true;
  const l = state.later;
  return !(l && l.version === s.available && Date.now() - l.at < LATER_MS);
}

export function later(): void {
  const version = state.status?.available;
  if (!version) return;
  state.later = { version, at: Date.now() };
  state.error = null;
  state.phoneMessage = null;
  try {
    localStorage.setItem(LATER_KEY, JSON.stringify(state.later));
  } catch {
    // A device that keeps nothing asks again next time it starts.
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;

/** **The one press.** Gets the newest version if it is not here yet, and installs it as soon as it
 *  is. Pressed again on a ready download — after a refusal — it goes straight to installing. On
 *  Android it opens the newest APK in the browser instead, and Android's installer does the rest. */
export async function update(): Promise<void> {
  state.error = null;
  state.phoneMessage = null;
  if (phoneUpdatesInBrowser()) {
    const r = downloadOnPhone();
    if (r === 'ok') {
      state.phoneMessage =
        'Your browser is downloading the new formicaria. When it finishes, tap Open and confirm: Android updates formicaria, and your notes stay as they are.';
    } else if (r) {
      state.error = r.replace(/^error:\s*/, '');
    }
    return;
  }
  state.going = true;
  const p = state.status?.progress;
  if (p?.stage === 'ready') return finish();
  if (!inFlight(p)) {
    try {
      state.status = await updateStart();
    } catch (e) {
      state.going = false;
      state.error = reason(e);
      return;
    }
  }
  watch();
}

/** Stop getting it. What already arrived is kept, so Update resumes. */
export async function stop(): Promise<void> {
  clearTimeout(timer);
  state.going = false;
  try {
    state.status = await updateCancel();
  } catch (e) {
    state.error = reason(e);
  }
}

/** Poll while getting it — the app has no streaming anywhere, and this is the cadence the
 *  assistant's own download uses. A failed poll is not a failed download. */
function watch() {
  clearTimeout(timer);
  timer = setTimeout(async () => {
    await refresh();
    const p = state.status?.progress;
    if (inFlight(p)) return watch();
    if (p?.stage === 'ready' && state.going) return finish();
    // Failed (the row says why and offers Try again), or cancelled elsewhere.
    state.going = false;
  }, 1500);
}

/** A download already under way when this page loaded: keep its progress moving. It does not
 *  install by itself once ready — that press belonged to a page that is gone. */
export function follow(): void {
  if (inFlight(state.status?.progress) && !state.going && !state.restarting) watch();
}

async function finish(): Promise<void> {
  // **What is being typed is saved before the program goes.** The editor's debounce lives in the
  // page, so only the page can flush it; the server then commits what is on disk (`settle`).
  await saveTyping();
  const was = state.status?.current ?? null;
  try {
    await updateApply();
    state.restarting = true;
    awaitRestart(was);
  } catch (e) {
    // Refused — a sync or backup still running, say. Nothing changed, so Update can be pressed again.
    state.error = reason(e);
  } finally {
    state.going = false;
  }
}

/** The event every open editor answers with its pending write (`NotePanel.svelte`). */
export const SAVE_NOW = 'fm-save-now';

/** Ask every open editor to write now, and wait for those writes — never longer than a few
 *  seconds: a write that hangs is not a reason to keep someone from an update, and what was typed
 *  is still in the editor's own page until the restart reloads it. */
async function saveTyping(): Promise<void> {
  if (typeof window === 'undefined') return;
  const writes: Promise<unknown>[] = [];
  window.dispatchEvent(new CustomEvent(SAVE_NOW, { detail: writes }));
  if (!writes.length) return;
  await Promise.race([Promise.allSettled(writes), new Promise((r) => setTimeout(r, 5000))]);
}

/** After an install or a *Go back*: wait for the other version to answer, then load the page from
 *  it. A missed answer is the restart itself, so it keeps asking; it stops only once the program
 *  that answers is no longer the one that was running (`was`: its version, or null for a build
 *  that has none). Before this, "this page reconnects by itself" stayed on screen over an app that
 *  had restarted long before (a user's desktop, 2026-10-11). */
export function awaitRestart(was: string | null): void {
  clearTimeout(timer);
  timer = setTimeout(async () => {
    try {
      if ((await updateStatus()).current !== was) return reloadPage();
    } catch {
      // Nobody answering yet: that is what a restart looks like.
    }
    awaitRestart(was);
  }, 1500);
}

let looking: ReturnType<typeof setTimeout> | undefined;

/** Keep the main window's picture current: now, again once the server's own startup check has had
 *  time to finish (it waits 5 s, then asks GitHub), and hourly after that, for a phone whose app
 *  stays alive for days. Every ask is local. Returns the way to stop. */
export function watchForUpdates(): () => void {
  const at = (ms: number) => {
    looking = setTimeout(async () => {
      if (!state.going && !state.restarting) await refresh().then(follow);
      at(60 * 60 * 1000);
    }, ms);
  };
  void refresh().then(follow);
  at(20_000);
  return () => clearTimeout(looking);
}

function reason(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Tests share one module; each starts from nothing. */
export function resetUpdater(): void {
  clearTimeout(timer);
  clearTimeout(looking);
  state.status = null;
  state.going = false;
  state.restarting = false;
  state.error = null;
  state.phoneMessage = null;
  state.later = storedLater();
}

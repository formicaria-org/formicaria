// **The music player, as a person uses it** (the owner, 2026-11-21: *"test whether the music layer is
// working sequentially … and check every button's actual behaviour"*). Shared by `usability-check.py`
// (formicaria's browser) and `phone-media-check.py` (the phone app): run with the Music screen open on a
// playlist of three songs, after its Play was pressed. Every check reads what the screen shows — the deck's
// "now playing", its position, each button's label or pressed state — never the code behind it.
// Called as an expression: `(%s)(titles)`, `titles` the three songs in the playlist's order.
async (titles) => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  // The deck on screen: another Music left open elsewhere has its own (2026-11-21).
  const deck = () => {
    const all = [...document.querySelectorAll('section[aria-label="music player"]')];
    return all.filter((d) => d.offsetParent !== null).pop() || all.pop() || null;
  };
  const q = (sel) => deck() && deck().querySelector(sel);
  const btn = (label) => q(`button[aria-label="${label}"]`);
  const title = () => ((q('.sr') || {}).textContent || '').replace(/^now playing:\s*/, '').trim();
  const pos = () => Number((q('input[aria-label="position in the song"]') || {}).value || 0);
  const max = () => Number((q('input[aria-label="position in the song"]') || {}).max || 0);
  const playing = () => !!btn('pause');
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const press = async (label) => { const b = btn(label); if (!b) return false; b.click(); await sleep(600); return true; };
  const seekTo = async (s) => {
    const r = q('input[aria-label="position in the song"]');
    if (!r) return false;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(r, String(s));
    r.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(600);
    return true;
  };
  const nearEnd = async () => { await until(() => max() > 3, 15); return seekTo(Math.max(0, max() - 2)); };
  const out = { steps: {} };
  const step = (name, ok, said) => { out.steps[name] = { ok: !!ok, said: said === undefined ? '' : String(said) }; };

  // ── one after another ────────────────────────────────────────────────────────────────────────────
  step('the first plays', await until(() => title() === titles[0] && playing(), 20), `${title()} playing=${playing()}`);
  await nearEnd();
  step('the second follows by itself', await until(() => title() === titles[1] && playing(), 15), `${title()} playing=${playing()}`);
  await nearEnd();
  step('the third follows by itself', await until(() => title() === titles[2] && playing(), 15), `${title()} playing=${playing()}`);
  await nearEnd();
  step('after the last, it stops (repeat off)', await until(() => !playing(), 15) && title() === titles[2], `${title()} playing=${playing()}`);

  // ── each button ──────────────────────────────────────────────────────────────────────────────────
  await press('previous song'); await press('previous song'); await press('previous song');
  await until(() => title() === titles[0], 10);
  if (!playing()) await press('play');
  step('play starts again from the list', await until(() => playing(), 10), `${title()} playing=${playing()}`);
  await sleep(1500);
  await press('pause');
  const held = pos();
  await sleep(2500);
  step('pause stops it where it is', !playing() && Math.abs(pos() - held) <= 1, `playing=${playing()} ${held}→${pos()}`);
  await press('play');
  await sleep(3000);
  step('play carries on from there', playing() && pos() > held, `playing=${playing()} ${held}→${pos()}`);
  await press('next song');
  step('next goes to the next song', await until(() => title() === titles[1], 8), title());
  await press('previous song');
  step('previous at a song\'s start goes to the one before', await until(() => title() === titles[0], 8), title());
  await seekTo(2);
  const a = pos();
  await press('forward 10 seconds');
  const b = pos();
  step('forward 10 seconds moves on ten', b - a >= 8 && b - a <= 12, `${a}→${b}`);
  await press('back 10 seconds');
  const c = pos();
  step('back 10 seconds moves back ten', b - c >= 8 && b - c <= 12, `${b}→${c}`);
  await press('stop');
  step('stop stops and goes back to the start', !playing() && pos() === 0, `playing=${playing()} pos=${pos()}`);
  await press('shuffle');
  const on = (q('button[aria-label="shuffle"]') || {}).getAttribute?.('aria-pressed');
  await press('shuffle');
  const off = (q('button[aria-label="shuffle"]') || {}).getAttribute?.('aria-pressed');
  step('shuffle switches on and off', on === 'true' && off === 'false', `${on}, ${off}`);
  // Repeat, pressed twice from "off" and read after each press: off → all → one.
  const repeatNow = () => (deck().querySelector('button[aria-label^="repeat: "]') || {}).getAttribute?.('aria-label');
  const repeats = [repeatNow()];
  await press(repeats[0]);
  repeats.push(repeatNow());
  await press(repeats[1]);
  repeats.push(repeatNow());
  // Now on "one": the song plays again instead of the next.
  const now = repeatNow();
  await press('play');
  await until(() => playing(), 8);
  const same = title();
  await nearEnd();
  await sleep(4000);
  step('repeat one plays the same song again', now === 'repeat: one' && title() === same && playing(), `${now}: ${same} → ${title()} playing=${playing()}`);
  // Round to "all": from the last song, the first comes next.
  await press('repeat: one'); // → off
  await press('repeat: off'); // → all
  const all = (deck().querySelector('button[aria-label^="repeat: "]') || {}).getAttribute?.('aria-label');
  for (let i = 0; i < 3 && title() !== titles[2]; i++) await press('next song');
  await until(() => title() === titles[2], 8);
  await nearEnd();
  step('repeat all goes from the last song back to the first', all === 'repeat: all' && await until(() => title() === titles[0] && playing(), 15), `${all}: ${title()}`);
  step('repeat cycles off, all, one', repeats.join(',') === 'repeat: off,repeat: all,repeat: one', repeats.join(','));
  await press('repeat: all'); // → one
  await press('repeat: one'); // → off
  await press('mute');
  const muted = btn('unmute') && btn('unmute').getAttribute('aria-pressed') === 'true';
  await press('unmute');
  step('mute switches the sound off and on', muted && !!btn('mute'), `muted=${muted}`);
  const vol = q('input[aria-label="volume"]');
  if (vol) {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(vol, '0.3');
    vol.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(600);
  }
  step('the volume keeps what it is set to', vol && Math.abs(Number(q('input[aria-label="volume"]').value) - 0.3) < 0.051, vol ? vol.value : 'no volume');
  if (vol) {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(vol, '1');
    vol.dispatchEvent(new Event('input', { bubbles: true }));
  }
  await press('queue');
  const listed = [...document.querySelectorAll('ol[aria-label="queue"] li, [aria-label="queue"] li')]
    .map((li) => (li.querySelector('.title') || li).textContent.trim());
  step('the queue lists the songs in order', titles.every((t, i) => (listed[i] || '').includes(t)), listed.slice(0, 5).join(' | '));
  await press('queue');
  await press('pause');
  return out;
}

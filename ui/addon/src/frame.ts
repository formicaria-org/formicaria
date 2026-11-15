// **formicaria beside the page, and on every new tab** (owner, 2026-11-15: a sidebar and a new tab
// that is formicaria). The add-on's own page, holding formicaria in a frame.
//
// formicaria refuses to be framed by anything (`frame-ancestors 'none'`) until this add-on has proven
// itself with the computer's key; then exactly this add-on's origin may frame it (`fm-serve`,
// `app_csp`). So the page first says hello with the key, and only then loads formicaria.
declare const browser: {
  storage: { managed: { get(): Promise<{ key?: string; server?: string }> } };
};

const say = (text: string) => {
  const p = document.getElementById('say') as HTMLElement;
  p.textContent = text;
  p.hidden = false;
};

async function open() {
  const m = await browser.storage.managed
    .get()
    .catch(() => ({}) as { key?: string; server?: string });
  if (!m.key || !m.server) {
    say('This works inside formicaria’s own browser. Open formicaria from its launcher.');
    return;
  }
  const server = m.server.replace(/\/+$/, '');
  try {
    const r = await fetch(`${server}/api/list_vaults`, {
      method: 'POST',
      headers: { 'X-Formicaria-Key': m.key, 'Content-Type': 'application/json' },
      body: '{}',
    });
    if (!r.ok) throw new Error(String(r.status));
  } catch {
    say('formicaria is not running. Start it from its launcher, then open this again.');
    return;
  }
  const where = new URLSearchParams(location.search).get('in');
  const f = document.createElement('iframe');
  f.src = `${server}/`;
  f.title = 'formicaria';
  f.setAttribute('allow', 'clipboard-write');
  document.body.appendChild(f);
  if (where === 'newtab') document.title = 'New tab — formicaria';
}
void open();

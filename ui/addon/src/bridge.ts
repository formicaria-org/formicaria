// **The bridge into formicaria's panel** (`decisions.md` 2026-11-15, late night). Runs only on
// formicaria's own pages (`127.0.0.1`; the background drops any other origin). It holds a port to the
// add-on's background and passes messages both ways: the background's to the app with
// `window.postMessage` (the app accepts them only from its own window), and the app's replies back.
// It carries nothing of its own and reads nothing from the page.
declare const browser: {
  runtime: {
    connect(d: { name: string }): {
      onMessage: { addListener(f: (m: unknown) => void): void };
      onDisconnect: { addListener(f: () => void): void };
      postMessage(m: unknown): void;
    };
  };
};

const port = browser.runtime.connect({ name: 'formicaria-bridge' });
// Tells the app it is inside formicaria's browser, so it reports the note in front and takes drops
// from pages; in any other browser it does neither.
document.documentElement.setAttribute('data-formicaria-bridge', '');
port.onMessage.addListener((m) => {
  window.postMessage({ formicaria_bridge: m }, location.origin);
});
window.addEventListener('message', (e: MessageEvent) => {
  if (e.source !== window || e.origin !== location.origin) return;
  const d = e.data as { formicaria_reply?: unknown; formicaria_up?: { up?: unknown } } | null;
  // A reply to the add-on's request, or a message of the app's own (the note in front, an
  // alternative chosen, a drop): both go up the same port.
  if (d?.formicaria_reply) port.postMessage(d.formicaria_reply);
  else if (typeof d?.formicaria_up?.up === 'string') port.postMessage(d.formicaria_up);
});
export {};

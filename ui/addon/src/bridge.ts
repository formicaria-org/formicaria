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
port.onMessage.addListener((m) => {
  window.postMessage({ formicaria_bridge: m }, location.origin);
});
window.addEventListener('message', (e: MessageEvent) => {
  if (e.source !== window || e.origin !== location.origin) return;
  const reply = (e.data as { formicaria_reply?: unknown } | null)?.formicaria_reply;
  if (reply) port.postMessage(reply);
});

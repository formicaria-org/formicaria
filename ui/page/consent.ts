// **Cookie pop-ups refused, on the phone's web pages** (`decisions.md` 2026-11-17, *privacy switches*):
// DuckDuckGo's own rules and engine (`@duckduckgo/autoconsent`, MPL-2.0), run in the page at its start
// by formicaria's page view while *Refuse cookie pop-ups* is on. Each banner it answers is counted for
// the Privacy sheet through the page view's counter (`__fmPagePrivacy.banner()`), nothing else.
// Built on its own (`addon/build.mjs`) into `dist/page/consent.js`; the computer's browser uses
// Firefox's own cookie banner reduction instead.
import AutoConsent, { filterCompactRules } from '@duckduckgo/autoconsent';
import compactRules from '@duckduckgo/autoconsent/rules/compact-rules.json';

type Counter = { banner?: () => void };
const w = window as unknown as {
  autoconsentReceiveMessage?: (m: unknown) => Promise<void>;
  __fmPagePrivacy?: Counter;
};

if (!w.autoconsentReceiveMessage) {
  const consent = new AutoConsent(async (m: { type: string; result?: boolean }) => {
    if (m.type === 'optOutResult' && m.result) w.__fmPagePrivacy?.banner?.();
  });
  w.autoconsentReceiveMessage = (m) => consent.receiveMessageCallback(m as never);
  consent.initialize(
    {
      enabled: true,
      autoAction: 'optOut',
      isMainWorld: true,
      enablePrehide: true,
      enableCosmeticRules: true,
      enableHeuristicDetection: true,
      logs: {
        lifecycle: false,
        rulesteps: false,
        detectionsteps: false,
        evals: false,
        errors: false,
        messages: false,
        waits: false,
      },
    } as never,
    {
      autoconsent: [],
      compact: filterCompactRules(compactRules as never, {
        url: location.href,
        mainFrame: window.top === window,
      }),
    } as never,
  );
}

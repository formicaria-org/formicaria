// **Cookie pop-ups on more sites, on a computer** (`privacy-plan-2026-11-19.md` 5.1): DuckDuckGo's
// own rules and engine (`@duckduckgo/autoconsent`, MPL-2.0), the same as the phone's
// (`ui/page/consent.ts`), run by formicaria's add-on in place of Consent-O-Matic. Registered by the
// background only while *Refuse cookie pop-ups* and *More sites* are both on (`syncConsent`), and
// never on a site whose protections are off; Firefox's own cookie banner reduction does the rest.
import AutoConsent, { filterCompactRules } from '@duckduckgo/autoconsent';
import compactRules from '@duckduckgo/autoconsent/rules/compact-rules.json';

const w = window as unknown as { __fmConsent?: boolean };
if (!w.__fmConsent) {
  w.__fmConsent = true;
  const consent = new AutoConsent(async () => {});
  consent.initialize(
    {
      enabled: true,
      autoAction: 'optOut',
      // A content script sees the page, not its scripts: rules that read the page's own scripts
      // are left to Firefox's reduction.
      isMainWorld: false,
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

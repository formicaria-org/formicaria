// **Procedural hide rules, on the phone's web pages** (`privacy-plan-2026-11-19.md` 3.4): the lists'
// rules a style sheet cannot say (`##div:has-text(Sponsored)`, `##.ad:upward(2)`, `##.x:remove()`),
// as Brave's engine gives them for a site (`Blocker.procedural`, a JSON array of
// `{selector: [{type, arg}…], action?}`), applied here, and again as the page changes (at most once a
// second). Built on its own (`addon/build.mjs`) into `dist/page/procedural.js`; formicaria's page view
// runs it at each page's start while *Block trackers* or *Block ads* is on.

export type Op = { type: string; arg: string };
export type Action = { type: 'remove' | 'style' | 'remove-attr' | 'remove-class'; arg?: string };
export type Rule = { selector: Op[]; action?: Action };

/** `/re/flags` → a regular expression; anything else → a plain substring test. */
function matcher(arg: string): (s: string) => boolean {
  const m = /^\/(.+)\/([a-z]*)$/s.exec(arg);
  if (m) {
    try {
      const re = new RegExp(m[1], m[2]);
      return (s) => re.test(s);
    } catch {
      return () => false;
    }
  }
  return (s) => s.includes(arg);
}

/** CSS that continues from an element: `> span`, ` .x`, `+ p` are relative to it. */
function from(e: Element, css: string): Element[] {
  const t = css.trimStart();
  const rel = /^[>+~]/.test(t) || css !== t;
  try {
    return [...e.querySelectorAll(rel ? `:scope ${t}` : css)];
  } catch {
    return [];
  }
}

function cssMatches(e: Element, arg: string, pseudo: string | null): boolean {
  const i = arg.indexOf(':');
  if (i < 0) return false;
  const prop = arg.slice(0, i).trim();
  const want = matcher(arg.slice(i + 1).trim());
  try {
    return want(getComputedStyle(e, pseudo).getPropertyValue(prop).trim());
  } catch {
    return false;
  }
}

function attrMatches(e: Element, arg: string): boolean {
  const [rawName, rawValue] = arg.split(/=(.*)/s);
  const unq = (s: string) => s.trim().replace(/^"(.*)"$/s, '$1');
  const name = matcher(unq(rawName ?? ''));
  const value = rawValue === undefined ? null : matcher(unq(rawValue));
  return [...e.attributes].some((a) => name(a.name) && (value === null || value(a.value)));
}

/** The elements a rule's selector reaches in `doc`. */
export function select(
  ops: Op[],
  doc: Document,
  where: string = location.pathname + location.search,
): Element[] {
  let set: Element[] | null = null;
  for (const op of ops) {
    const cur: Element[] = set ?? [doc.documentElement];
    switch (op.type) {
      case 'css-selector':
        set =
          set === null
            ? (() => {
                try {
                  return [...doc.querySelectorAll(op.arg)];
                } catch {
                  return [];
                }
              })()
            : cur.flatMap((e) => from(e, op.arg));
        break;
      case 'has-text': {
        const m = matcher(op.arg);
        set = cur.filter((e) => m(e.textContent ?? ''));
        break;
      }
      case 'min-text-length': {
        const n = Number(op.arg) || 0;
        set = cur.filter((e) => (e.textContent ?? '').length >= n);
        break;
      }
      case 'matches-attr':
        set = cur.filter((e) => attrMatches(e, op.arg));
        break;
      case 'matches-css':
        set = cur.filter((e) => cssMatches(e, op.arg, null));
        break;
      case 'matches-css-before':
        set = cur.filter((e) => cssMatches(e, op.arg, '::before'));
        break;
      case 'matches-css-after':
        set = cur.filter((e) => cssMatches(e, op.arg, '::after'));
        break;
      case 'matches-path':
        set = matcher(op.arg)(where) ? cur : [];
        break;
      case 'upward': {
        const n = Number(op.arg);
        set = cur
          .map((e) => {
            if (Number.isInteger(n) && n > 0) {
              let up: Element | null = e;
              for (let i = 0; i < n && up; i++) up = up.parentElement;
              return up;
            }
            try {
              return e.parentElement?.closest(op.arg) ?? null;
            } catch {
              return null;
            }
          })
          .filter((e): e is Element => !!e);
        break;
      }
      case 'xpath':
        set = cur.flatMap((e) => {
          try {
            const r = doc.evaluate(op.arg, e, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
            const out: Element[] = [];
            for (let i = 0; i < r.snapshotLength; i++) {
              const x = r.snapshotItem(i);
              if (x instanceof Element) out.push(x);
            }
            return out;
          } catch {
            return [];
          }
        });
        break;
      default:
        // An operator this does not know: the rule reaches nothing rather than too much.
        return [];
    }
  }
  return [...new Set(set ?? [])];
}

/** Apply the rules to `doc`; answers how many elements were acted on. */
export function apply(rules: Rule[], doc: Document, where?: string): number {
  let n = 0;
  for (const r of rules) {
    for (const e of select(r.selector, doc, where)) {
      const a = r.action;
      if (!a) (e as HTMLElement).style?.setProperty('display', 'none', 'important');
      else if (a.type === 'remove') e.remove();
      else if (a.type === 'style' && a.arg)
        e.setAttribute('style', `${e.getAttribute('style') ?? ''};${a.arg}`);
      else if (a.type === 'remove-attr' && a.arg) e.removeAttribute(a.arg);
      else if (a.type === 'remove-class' && a.arg) e.classList.remove(a.arg);
      else continue;
      n++;
    }
  }
  return n;
}

type Bridge = { procedural?: (url: string) => string };
const w = globalThis as unknown as { __fmPagePrivacy?: Bridge; __fmProcedural?: boolean };

if (typeof window !== 'undefined' && w.__fmPagePrivacy?.procedural && !w.__fmProcedural) {
  w.__fmProcedural = true;
  let rules: Rule[] = [];
  try {
    rules = JSON.parse(w.__fmPagePrivacy.procedural(location.href)) as Rule[];
  } catch {
    rules = [];
  }
  if (rules.length) {
    const run = () => {
      try {
        apply(rules, document);
      } catch {
        /* a page that fights back: the rest of it still loads */
      }
    };
    const start = () => {
      run();
      let t: ReturnType<typeof setTimeout> | 0 = 0;
      new MutationObserver(() => {
        if (t) return;
        t = setTimeout(() => {
          t = 0;
          run();
        }, 1000);
      }).observe(document.documentElement, { childList: true, subtree: true });
    };
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
    else start();
  }
}

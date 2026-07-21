// The closed vocabularies for the read-view's decorative Markdown extensions — the **one source
// of truth**. The `marked` extensions in `render.ts` emit a class *only* for a member of these
// sets (an unknown value is left as literal text, never a class), and `app.css` styles each
// member with a **semantic theme token** (never a colour name or hex), so a note stays truthful
// across themes. The sanitizer never learns these: the extensions emit DOMPurify-default-allowed
// `<mark>`/`<span>`/`<div>` + `class`, so adding a member is one entry here + one CSS rule, and
// touches no security config. Enforcement of the closed set is entirely in the emitter (DOMPurify
// does not validate class *values*).

/** Inline colour tokens for `[text]{.token}`. **Semantic intent, not colour** — `accent`/`ok`/… —
 *  so the meaning survives theme changes (`app.css` maps each to a token that is themed). */
export const TEXT_TOKENS = ['accent', 'info', 'ok', 'warn', 'muted'] as const;
export type TextToken = (typeof TEXT_TOKENS)[number];

/** Callout kinds for `> [!type]` blockquotes. GitHub/Obsidian standard; unknown kinds stay plain
 *  blockquotes, so the bytes always degrade to readable Markdown. */
export const CALLOUT_TYPES = ['note', 'tip', 'info', 'warning', 'danger', 'quote'] as const;
export type CalloutType = (typeof CALLOUT_TYPES)[number];

/** A byte count as the shortest string a person would write.
 *
 *  **The grammar is the backend's, deliberately.** `Descriptor`'s parser accepts `"2MB"` or a
 *  raw byte count, so what a field shows is what you could type back into it — and what one
 *  panel shows is what the other shows. It lived as a private function inside `SettingsPanel`
 *  until the backup panel needed to say the same number in a sentence; a second copy is how the
 *  two surfaces would start disagreeing about what "2MB" means. */
/** The largest attachment this app will put into git, whatever a vault asks for.
 *
 *  **Must equal `GIT_ASSETS_CEILING` in `crates/fm-core/src/descriptor.rs`**, which is the
 *  authority — this copy exists only so the form can warn before the backend refuses. `ci/checks.sh`
 *  fails when the two numbers disagree, because two constants that must agree and no guard is
 *  precisely how they stop agreeing.
 *
 *  100 MB decimal, a little inside GitHub's 100 MiB wall. There is no git-lfs here, so every
 *  attachment under a vault's limit is a full blob in permanent history. */
export const GIT_ASSETS_CEILING = 100_000_000;

/** Where the form starts warning rather than simply accepting. Below this a limit is unremarkable;
 *  between here and the ceiling it still works, and the user should know what it costs — hosts warn
 *  above 50 MB, clones pay for it forever, and git cannot take it back. A UI threshold only: the
 *  backend has no opinion about it, which is why it is not mirrored in Rust. */
export const GIT_ASSETS_WARN = 50_000_000;

export function humanSize(bytes: number): string {
  for (const [unit, mult] of [
    ['GB', 1e9],
    ['MB', 1e6],
    ['kB', 1e3],
  ] as const) {
    const v = bytes / mult;
    if (v >= 1)
      return Math.abs(v % 1) < 0.05 ? `${Math.round(v)}${unit}` : `${v.toFixed(1)}${unit}`;
  }
  return `${bytes}B`;
}

/** A byte count as the shortest string a person would write.
 *
 *  **The grammar is the backend's, deliberately.** `Descriptor`'s parser accepts `"2MB"` or a
 *  raw byte count, so what a field shows is what you could type back into it — and what one
 *  panel shows is what the other shows. It lived as a private function inside `SettingsPanel`
 *  until the backup panel needed to say the same number in a sentence; a second copy is how the
 *  two surfaces would start disagreeing about what "2MB" means. */
export function humanSize(bytes: number): string {
  for (const [unit, mult] of [
    ['GB', 1e9],
    ['MB', 1e6],
    ['kB', 1e3],
  ] as const) {
    const v = bytes / mult;
    if (v >= 1) return Math.abs(v % 1) < 0.05 ? `${Math.round(v)}${unit}` : `${v.toFixed(1)}${unit}`;
  }
  return `${bytes}B`;
}

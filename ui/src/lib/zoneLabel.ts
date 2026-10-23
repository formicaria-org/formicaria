// The time zone the agenda shows its times in — `decisions.md` 2026-10-23, *every device shows the
// agenda on its own zone*. It is meant to be **observable, and minimal**: one short label
// beside the agenda's M/W/L switch, with the whole story in its tooltip.

/** This device's zone, as the browser (or a phone's webview) reports it. */
export function deviceZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  } catch {
    return 'UTC';
  }
}

/** `+02:00`-style offset of `zone` at `when`, as `GMT+2` / `GMT+5:30` / `GMT`. */
export function shortOffset(zone: string, when: Date = new Date()): string {
  try {
    const part = new Intl.DateTimeFormat('en-US', { timeZone: zone, timeZoneName: 'shortOffset' })
      .formatToParts(when)
      .find((p) => p.type === 'timeZoneName')?.value;
    return part ?? '';
  } catch {
    return '';
  }
}

/** `Europe/Berlin` → `Berlin`; `America/Argentina/Buenos_Aires` → `Buenos Aires`. */
export function city(zone: string): string {
  return (zone.split('/').pop() ?? zone).replace(/_/g, ' ');
}

/** The label (`Berlin · GMT+2`) and its tooltip, given the clocks the vaults keep their times on. */
export function zoneLabel(
  zone: string,
  vaultClocks: (string | null | undefined)[],
  when: Date = new Date(),
): { text: string; title: string } {
  const off = shortOffset(zone, when);
  const text = off && off !== city(zone) ? `${city(zone)} · ${off}` : city(zone);
  const kept = [...new Set(vaultClocks.filter((c): c is string => !!c))];
  const title =
    `Times are shown on this device's clock: ${zone}.` +
    (kept.length ? ` Your notes keep theirs on ${kept.join(', ')}, and each device converts.` : '');
  return { text, title };
}

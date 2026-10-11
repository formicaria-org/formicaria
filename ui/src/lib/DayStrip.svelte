<script lang="ts">
  // A fortnight as a row of bars — how many notes fall on each day — where a tap keeps that day's
  // notes and a second tap lets them all back. The same bars the trackers draw
  // (`decisions.md#ui`, 2026-10-12, the visual pass); what is counted is the caller's.
  import Bars from './charts/Bars.svelte';
  import { dayLabel, dayLetter, dayNumber, today } from './units';

  interface Props {
    caption: string;
    days: string[];
    /** The day each note falls on; a note with none is not counted. */
    on: (string | null)[];
    chosen: string | null;
    onpick: (day: string | null) => void;
  }
  let { caption, days, on, chosen, onpick }: Props = $props();

  const counts = $derived.by(() => {
    const n = new Map<string, number>();
    for (const d of on) if (d) n.set(d, (n.get(d) ?? 0) + 1);
    return n;
  });
  const any = $derived(days.some((d) => counts.has(d)));
</script>

{#if any}
  <section class="day-strip" aria-label={caption}>
    <p>{caption}{chosen ? ` — ${dayLabel(chosen)}` : ''}</p>
    <Bars
      height={64}
      selected={chosen ?? undefined}
      bars={days.map((d) => ({
        key: d,
        value: counts.get(d) ?? 0,
        label: `${dayLabel(d)}: ${counts.get(d) ?? 0}`,
        tick:
          d === today()
            ? '•'
            : dayNumber(d) === 1 || d === days[0]
              ? String(dayNumber(d))
              : dayLetter(d),
      }))}
      onpick={(d) => onpick(d === chosen ? null : d)}
    />
  </section>
{/if}

<style>
  .day-strip {
    padding: var(--space-2) var(--space-4) 0;
  }
  p {
    margin: 0 0 var(--space-1);
    color: var(--text-muted);
    font-size: 0.8rem;
  }
</style>

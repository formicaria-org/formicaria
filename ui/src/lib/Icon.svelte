<script lang="ts">
  // One inline-SVG icon component — Feather/Lucide 24×24 stroke geometry, zero
  // deps (the perf budget forbids an icon font/megabundle). Add an entry to ICONS
  // to add an icon; callers reference it by name. Decorative by default
  // (aria-hidden); pass a `title` for a labelled icon.
  let {
    name,
    size = 18,
    title,
  }: { name: string; size?: number; title?: string } = $props();

  // Each value is the inner SVG markup (paths/circles/lines) in 24×24 space.
  const ICONS: Record<string, string> = {
    board: '<rect x="3" y="4" width="5" height="16" rx="1"/><rect x="10" y="4" width="5" height="10" rx="1"/><rect x="17" y="4" width="4" height="16" rx="1"/>',
    calendar: '<rect x="3" y="4" width="18" height="17" rx="2"/><path d="M3 9h18M8 2v4M16 2v4"/>',
    timeline: '<circle cx="5" cy="6" r="1.6"/><circle cx="5" cy="18" r="1.6"/><path d="M5 8v8M10 6h9M10 18h9"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    command: '<path d="M8 6a2 2 0 1 0 2 2v8a2 2 0 1 0 2-2H8a2 2 0 1 0-2 2V8a2 2 0 1 0 2 2z"/>',
    sun: '<circle cx="12" cy="12" r="4.2"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M19.1 4.9l-1.4 1.4M6.3 17.7l-1.4 1.4"/>',
    moon: '<path d="M21 12.8A8.5 8.5 0 1 1 11.2 3a6.6 6.6 0 0 0 9.8 9.8z"/>',
    close: '<path d="M6 6l12 12M18 6L6 18"/>',
    chevronLeft: '<path d="M15 6l-6 6 6 6"/>',
    chevronRight: '<path d="M9 6l6 6-6 6"/>',
    backup: '<path d="M4 15a5 5 0 0 1 1.2-9.8A6 6 0 0 1 17 6a4.5 4.5 0 0 1 1 8.9"/><path d="M12 12v7M9 16l3 3 3-3"/>',
    inbox: '<path d="M4 13h4l2 3h4l2-3h4"/><path d="M5 5h14l2 8v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5z"/>',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={title ? 'img' : undefined}
  aria-hidden={title ? undefined : 'true'}
  aria-label={title}
>
  {#if title}<title>{title}</title>{/if}
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html ICONS[name] ?? ''}
</svg>

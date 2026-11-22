<script lang="ts">
  // **A section's notes, as a table or a gallery** (the shell, P2a): the generic views every section
  // has beside its own, with the same verb on each, *Filter*. The notes come from `section_notes`,
  // the section's own rule (`sections.rs`), read again whenever the vaults change.
  import { onMount } from 'svelte';
  import * as ipc from '../lib/ipc';
  import type { ObjectMeta, Section, SectionView } from '../lib/types';
  import Table from '../renderers/Table.svelte';
  import Gallery from '../renderers/Gallery.svelte';

  let {
    section,
    view,
    onopen,
  }: { section: Section; view: SectionView; onopen: (id: string) => void } = $props();

  let cards = $state<ObjectMeta[]>([]);
  let failed = $state('');
  let filter = $state('');

  async function read() {
    try {
      cards = await ipc.sectionNotes(section.key);
      failed = '';
    } catch (e) {
      failed = String(e);
    }
  }
  $effect(() => {
    void section.key;
    void read();
  });
  onMount(() => ipc.listenChanges(() => void read()));

  const fold = (t: string) => t.toLocaleLowerCase().normalize('NFD').replace(/\p{M}/gu, '');
  const shown = $derived.by(() => {
    const words = fold(filter).split(/\s+/).filter(Boolean);
    if (!words.length) return cards;
    return cards.filter((c) => {
      const hay = fold([c.title ?? '', c.preview, c.tags.join(' ')].join(' '));
      return words.every((w) => hay.includes(w));
    });
  });
</script>

<div class="section-notes">
  <div class="verbs">
    <input
      type="search"
      placeholder={`Filter ${section.label.toLowerCase()}`}
      aria-label={`filter ${section.label}`}
      bind:value={filter}
    />
    <span class="count">{shown.length} of {cards.length}</span>
  </div>
  {#if failed}<p class="error" role="alert">{failed}</p>{/if}
  {#if view.renderer === 'gallery'}
    <Gallery cards={shown} {onopen} />
  {:else}
    <Table cards={shown} columns={view.columns ?? []} {onopen} />
  {/if}
</div>

<style>
  .section-notes {
    height: 100%;
    overflow: auto;
  }
  .verbs {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) 0;
  }
  .verbs input {
    flex: 0 1 18rem;
    font: inherit;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
  }
  .count {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .error {
    color: var(--danger-fg);
    padding: 0 var(--space-4);
  }
</style>

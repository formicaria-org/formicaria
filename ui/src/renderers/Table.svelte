<script lang="ts">
  // **A section as a table** (the shell, P2a; `unifying-home-research-2026-11-22.md` §10.2): one row
  // per note, a column per property the section names (`columns:` in its `.kind`), then when it was
  // last changed. A header sorts by its column, and again the other way. Generic: it reads property
  // names it is given and knows none of them (renderers stay literal-free).
  import type { ObjectMeta } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';
  import { relativeTime } from '../lib/stamp';

  let {
    cards,
    columns = [],
    onopen,
  }: { cards: ObjectMeta[]; columns?: string[]; onopen: (id: string) => void } = $props();

  const cols = $derived(columns.filter((c) => c !== 'title'));
  let by = $state<string>('updated');
  let down = $state(true);

  const valueOf = (c: ObjectMeta, key: string): string => {
    if (key === 'title') return c.title ?? c.preview ?? '';
    if (key === 'updated') return c.updated;
    if (key === 'tags') return c.tags.join(', ');
    const v = (c.props ?? {})[key] ?? (c as unknown as Record<string, unknown>)[key];
    return v === null || v === undefined ? '' : String(v);
  };
  const shown = (c: ObjectMeta, key: string) =>
    key === 'updated' ? relativeTime(c.updated) : valueOf(c, key);
  const label = (key: string) =>
    key === 'updated' ? 'Changed' : key.replace(/_s$/, '').replace(/_/g, ' ');

  const rows = $derived(
    [...cards].sort((a, b) => {
      const x = valueOf(a, by);
      const y = valueOf(b, by);
      const n = Number(x) - Number(y);
      const c = !Number.isNaN(n) && x !== '' && y !== '' ? n : x.localeCompare(y);
      return down ? -c : c;
    }),
  );
  function sortBy(key: string) {
    if (by === key) down = !down;
    else {
      by = key;
      down = key === 'updated';
    }
  }
</script>

{#if !cards.length}
  <EmptyState icon="list" title="Nothing here yet" hint="Press ＋ New to add the first one." />
{:else}
  <div class="wrap">
    <table>
      <thead>
        <tr>
          {#each ['title', ...cols, 'updated'] as key (key)}
            <th aria-sort={by === key ? (down ? 'descending' : 'ascending') : undefined}>
              <button type="button" onclick={() => sortBy(key)}
                >{key === 'title' ? 'Name' : label(key)}{by === key
                  ? down
                    ? ' ↓'
                    : ' ↑'
                  : ''}</button
              >
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as c (c.id)}
          <tr>
            <td class="name">
              <button type="button" class="open" onclick={() => onopen(c.id)}
                >{c.title || c.preview || 'Untitled'}</button
              >
            </td>
            {#each cols as key (key)}<td>{shown(c, key)}</td>{/each}
            <td class="when">{shown(c, 'updated')}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .wrap {
    overflow-x: auto;
    padding: var(--space-3) var(--space-4);
  }
  table {
    border-collapse: collapse;
    width: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  th,
  td {
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    font-size: var(--text-sm);
    white-space: nowrap;
  }
  th button {
    font: inherit;
    font-weight: 600;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  td.name {
    white-space: normal;
    min-width: 12rem;
  }
  .open {
    font: inherit;
    color: var(--text);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .open:hover {
    text-decoration: underline;
  }
  .when {
    color: var(--text-muted);
  }
</style>

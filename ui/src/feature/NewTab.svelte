<script lang="ts">
  // **A new tab suggests** (`decisions.md` 2026-11-17, *one frame*; Teams' add-a-tab, Vivaldi's Speed
  // Dial): the features as large buttons, a new note, and one field for a web address or a search.
  // Choosing fills this tab.
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { FEATURES, webTarget } from '../lib/features';
  import * as ipc from '../lib/ipc';

  let { go = (url: string) => location.assign(url) }: { go?: (url: string) => void } = $props();

  let typed = $state('');
  let field = $state<HTMLInputElement>();
  const web = $derived(webTarget(typed));

  async function newNote() {
    const [made] = await ipc.createNotes([{ title: '', body: '', tags: [], start: '', props: {} }]);
    if (made) go(`/f/note/${made.id}`);
  }
  onMount(() => field?.focus());
</script>

<div class="new-tab">
  <input
    bind:this={field}
    bind:value={typed}
    type="search"
    placeholder="A web address, or words to search the web"
    aria-label="a web address or a search"
    onkeydown={(e) => e.key === 'Enter' && web && go(web.url)}
  />
  {#if web}
    <button type="button" class="web" onclick={() => go(web.url)}>{web.label}</button>
  {/if}
  <ul class="features" aria-label="features">
    <li>
      <button type="button" onclick={() => void newNote()}
        ><Icon name="plus" size={22} /><span>New note</span></button
      >
    </li>
    {#each FEATURES as f (f.key)}
      <li>
        <button type="button" onclick={() => go(f.url)}
          ><Icon name={f.icon} size={22} /><span>{f.label}</span></button
        >
      </li>
    {/each}
  </ul>
</div>

<style>
  .new-tab {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: 10vh var(--space-3) var(--space-3);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  input {
    width: min(36rem, 100%);
    font-size: 1.1rem;
    padding: var(--space-2) var(--space-3);
  }
  .web {
    width: min(36rem, 100%);
    text-align: start;
    color: var(--accent, LinkText);
  }
  .features {
    list-style: none;
    margin: var(--space-3) 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
    gap: var(--space-2);
    width: min(40rem, 100%);
  }
  .features button {
    width: 100%;
    min-height: 5.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    border-radius: 12px;
  }
</style>

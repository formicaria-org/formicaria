<script lang="ts">
  // **A watch's activity files, read into the training log** (`decisions.md` 2026-11-20). On a
  // computer only: the person types the folder they copied the watch's files to (or the one another
  // app exports them to), sees what would be added, then imports. The folder is only read.
  //
  // Every verdict is the server's (`fit_preview` / `import_fit`, `crates/fm-app/src/watch.rs`): this
  // panel shows what it is told and adds no opinion of its own.
  //
  // **On the phone** (`decisions.md` 2026-11-20, *The phone takes watch files*) there is no path to
  // type: a `.fit` shared to formicaria is imported, and one folder the person grants — the folder
  // their watch's app exports to — is read on opening and on every return to the app
  // (`watchFiles.svelte.ts`). The typed path stays a computer's.
  import ManualLink from './ManualLink.svelte';
  import { onMount } from 'svelte';
  import { fitPreview, importFit } from './ipc';
  import { vaultList, refreshVaults, labelFor } from './vaults.svelte';
  import type { FitReport } from './types';
  import {
    watchFiles,
    chooseFolder,
    forgetFolder,
    lookNow,
    folderState,
  } from './watchFiles.svelte';

  let { phone = false }: { phone?: boolean } = $props();
  const folderNow = $derived(watchFiles.folder);

  let folder = $state('');
  let vault = $state('');
  let preview = $state<FitReport | null>(null);
  let done = $state<FitReport | null>(null);
  let busy = $state(false);
  let failed = $state('');

  const vaults = $derived(vaultList() ?? []);

  onMount(() => {
    if (!phone) {
      void refreshVaults();
      return;
    }
    folderState();
    const on = () => folderState();
    window.addEventListener('fm-watch', on);
    return () => window.removeEventListener('fm-watch', on);
  });

  async function check() {
    if (busy) return;
    busy = true;
    failed = '';
    done = null;
    try {
      preview = await fitPreview(folder, vault);
    } catch (e) {
      preview = null;
      failed = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function run() {
    if (busy || !preview?.ok) return;
    busy = true;
    failed = '';
    try {
      done = await importFit(folder, vault);
      preview = null;
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function km(m: number | undefined): string {
    return m ? ` · ${(m / 1000).toFixed(1)} km` : '';
  }

  function plural(n: number, one: string, many: string): string {
    return `${n} ${n === 1 ? one : many}`;
  }
</script>

<section aria-label="watch files">
  <h3>Watch files (.fit)</h3>
  <ManualLink page="user/trackers.html" section="watch-files" />
  {#if phone}
    <p class="muted">
      Activities from a sports watch, read from the files its app writes on this phone —
      Gadgetbridge, for example. formicaria does not talk to the watch and signs in nowhere. Share a
      <code>.fit</code> file to formicaria and it goes into Fitness, or let formicaria read the folder
      the watch's app exports to.
    </p>
    {#if folderNow?.folder}
      <p>
        Reading <strong>{folderNow.folder}</strong>: new <code>.fit</code> files there are added each
        time you open formicaria. It is only read.
      </p>
      {#if !folderNow.readable}
        <p class="err" role="alert">
          That folder is no longer readable — Android took the permission back, or the folder moved.
          Choose it again.
        </p>
      {/if}
      {#if folderNow.waiting}
        <p class="muted">
          {plural(folderNow.waiting, 'file is', 'files are')} still being written, and will be taken next
          time.
        </p>
      {/if}
      {#if folderNow.tooLarge}
        <p class="muted">
          {plural(folderNow.tooLarge, 'file is', 'files are')} larger than a watch's activity file, and
          left out.
        </p>
      {/if}
      <div class="actions">
        <button type="button" onclick={lookNow} disabled={folderNow.looking}>
          {folderNow.looking ? 'Looking…' : 'Look now'}
        </button>
        <button type="button" onclick={chooseFolder}>Choose another folder</button>
        <button type="button" onclick={forgetFolder}>Stop reading it</button>
      </div>
    {:else}
      <div class="actions">
        <button type="button" onclick={chooseFolder}>
          Choose the folder your watch app exports to
        </button>
      </div>
      <p class="muted">
        In Gadgetbridge, turn on its automatic export of activity files and choose the same folder
        here. formicaria asks Android to read that one folder only, and keeps the permission on this
        phone.
      </p>
    {/if}
    {#if watchFiles.last}
      {@const l = watchFiles.last}
      <div class="report" aria-live="polite">
        {#if l.error}
          <p class="err">Not imported: {l.error}</p>
        {:else if l.report}
          <p>
            {l.from === 'share' ? 'Shared to formicaria' : 'From the folder'}:
            {#if l.report.written}
              added {plural(l.report.activities, 'activity', 'activities')}{l.report.sets
                ? ` and ${plural(l.report.sets, 'set', 'sets')}`
                : ''}.
            {:else}
              nothing new.
            {/if}
          </p>
          {@render notes(l.report)}
        {/if}
      </div>
    {/if}
  {:else}
    <p class="muted">
      Activities from a sports watch, read from the files it writes: copy them to a folder on this
      computer over USB, or use the folder another app exports them to. formicaria does not talk to
      the watch and signs in nowhere; the folder is only read.
    </p>
    <label class="field">
      <span>Folder</span>
      <input
        bind:value={folder}
        placeholder="~/watch/Activity"
        autocomplete="off"
        spellcheck="false"
        autocapitalize="off"
        disabled={busy}
        oninput={() => {
          preview = null;
          done = null;
        }}
      />
    </label>
    <!-- Asked once there is a folder to import: until then a second list of every vault would sit
       in Settings beside the Vaults category's own. Named as the rest of Settings names them. -->
    {#if vaults.length > 1 && folder.trim()}
      <label class="field">
        <span>Into</span>
        <select bind:value={vault} disabled={busy} onchange={() => (preview = null)}>
          <option value="">{labelFor(vaults[0]?.name)}</option>
          {#each vaults.slice(1) as v (v.name)}
            <option value={v.name}>{labelFor(v.name)}</option>
          {/each}
        </select>
      </label>
    {/if}
    <div class="actions">
      <button type="button" onclick={() => void check()} disabled={busy || !folder.trim()}>
        Check
      </button>
      <button type="button" onclick={() => void run()} disabled={busy || !preview?.ok}>
        Import
      </button>
    </div>

    {#if failed}
      <p class="err" role="alert">{failed}</p>
    {/if}

    {#if preview}
      <div class="report" aria-live="polite">
        {#if preview.problem}
          <p class="err">{preview.problem}</p>
        {:else}
          <p>
            {plural(preview.files, 'file', 'files')} found.
            {#if preview.activities}
              Would add {plural(preview.activities, 'activity', 'activities')}{preview.sets
                ? ` and ${plural(preview.sets, 'set', 'sets')}`
                : ''}.
            {:else}
              Nothing new to add.
            {/if}
          </p>
          {#if preview.items.length}
            <ul class="items">
              {#each preview.items as it, i (i)}
                <li>
                  {it.title} · {it.day} · {it.minutes} min{km(it.distanceM)}{it.sets
                    ? ` · ${plural(it.sets, 'set', 'sets')}`
                    : ''}
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
        {@render notes(preview)}
      </div>
    {/if}

    {#if done}
      <div class="report" aria-live="polite">
        <p>
          {#if done.written}
            Added {plural(done.activities, 'activity', 'activities')}{done.sets
              ? ` and ${plural(done.sets, 'set', 'sets')}`
              : ''} to {done.vault}. They are in Fitness, on the day each was done.
          {:else}
            Nothing new was added.
          {/if}
        </p>
        {@render notes(done)}
        {#each done.warnings as w, i (i)}
          <p class="err">{w}</p>
        {/each}
      </div>
    {/if}
  {/if}
</section>

{#snippet notes(r: FitReport)}
  {#if r.known}
    <p class="muted">
      {plural(r.known, 'file was', 'files were')} imported before and left as they are.
    </p>
  {/if}
  {#each r.otherVersions as v, i (i)}
    <p class="muted">
      {v.file} is another version of “{v.note}”, imported before{v.edited
        ? ', which you have changed since'
        : ''}. It was not imported, and nothing was overwritten.
    </p>
  {/each}
  {#each r.merges as m, i (i)}
    <p class="muted">
      {m.file} looks like the same activity as “{m.with}”{m.day ? ` (${m.day})` : ''}, so it was not
      added. If it is a different one, add it by hand.
    </p>
  {/each}
  {#each r.refused as f, i (i)}
    <p class="err">{f.file}: {f.reason}</p>
  {/each}
  {#if r.notActivities.length}
    <p class="muted">
      Left out, not activities: {r.notActivities.join(', ')}.
    </p>
  {/if}
{/snippet}

<style>
  section {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }
  .muted {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin: var(--space-2) 0 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .err {
    color: var(--danger-fg);
    overflow-wrap: anywhere;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-2);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .report {
    margin-top: var(--space-2);
  }
  .items {
    margin: var(--space-1) 0 0;
    padding-left: var(--space-4);
    font-size: var(--text-sm);
  }
</style>

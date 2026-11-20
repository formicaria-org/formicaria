<script lang="ts">
  // **The usage diary** in Settings (`fm_app::diary`, `decisions.md` 2026-11-20): on or off, how
  // much is kept and since when, and *Delete it*. Kept on this device; a phone's goes to the
  // person's computer with their notes' sync, nowhere else.
  import { onMount } from 'svelte';
  import { diaryForget, diarySet, diaryStatus, type DiaryStatus } from './ipc';
  import { humanSize } from './size';

  let { phone = false }: { phone?: boolean } = $props();
  let s = $state<DiaryStatus | null>(null);
  let err = $state('');

  const run = (p: Promise<DiaryStatus>) =>
    p.then(
      (v) => {
        s = v;
        err = '';
      },
      (e) => (err = String(e)),
    );
  onMount(() => void run(diaryStatus()));
</script>

{#if s}
  <div class="diary">
    <label class="row">
      <span class="text">
        <span class="name">Usage diary</span>
        <span class="muted">
          What formicaria does on this {phone ? 'phone' : 'computer'}, to make it faster and
          lighter: memory and processor, starts and stops, errors, which features are used, counted
          by name. Never a note's text or title, a web address, a search or a mail. Kept on this
          device{phone ? ' and carried to your computer with your notes' : ''}; never sent anywhere
          else.
        </span>
      </span>
      <input
        type="checkbox"
        role="switch"
        aria-label="Usage diary"
        checked={s.on}
        onchange={(e) => run(diarySet((e.currentTarget as HTMLInputElement).checked))}
      />
    </label>
    <p class="muted kept">
      {#if s.bytes}
        {humanSize(s.bytes)} over {s.days} day{s.days === 1 ? '' : 's'}{s.since
          ? `, since ${s.since}`
          : ''}; kept {s.keep_days ?? 90} days.
      {:else}
        Nothing kept yet.
      {/if}
      <button type="button" disabled={!s.bytes} onclick={() => run(diaryForget())}>Delete it</button
      >
    </p>
    {#if err}<p class="error" role="alert">{err}</p>{/if}
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.35rem 0;
  }
  .text {
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 600;
  }
  .kept {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  .error {
    color: var(--danger, #c00);
  }
</style>

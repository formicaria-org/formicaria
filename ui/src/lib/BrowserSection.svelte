<script lang="ts">
  // **formicaria's own browser, on a computer** (`decisions.md` 2026-11-15). Firefox ESR from Mozilla,
  // set up in formicaria's settings folder with formicaria's add-on: a *Keep* button on every page
  // that turns the page into a note — a recipe, a paper, an article, a video or a song — with a copy
  // of the page. Asked for, never assumed: nothing is downloaded until the person presses Set up.
  import { onDestroy, onMount } from 'svelte';
  import ManualLink from './ManualLink.svelte';
  import { browserOpen, browserSet, browserSetup, browserStatus } from './ipc';
  import type { BrowserStatus } from './types';

  let st = $state<BrowserStatus | null>(null);
  let said = $state('');
  let timer: ReturnType<typeof setInterval> | undefined;

  const mb = (n: number) => `${Math.round(n / (1024 * 1024))} MB`;

  async function load() {
    try {
      st = await browserStatus();
    } catch {
      // A build without the browser (a phone, or a server built without it): no section at all.
      st = null;
    }
    if (st?.busy && !timer) timer = setInterval(load, 1000);
    if (!st?.busy && timer) {
      clearInterval(timer);
      timer = undefined;
    }
  }
  onMount(load);
  onDestroy(() => timer && clearInterval(timer));

  async function setUp() {
    said = '';
    try {
      st = await browserSetup();
      if (!timer) timer = setInterval(load, 1000);
    } catch (e) {
      said = e instanceof Error ? e.message : String(e);
    }
  }
  async function set(p: { open_in_it?: boolean; block_ads?: boolean; web_push?: boolean }) {
    st = await browserSet(p);
  }
  async function openNow() {
    said = '';
    try {
      st = await browserOpen();
      said = 'Opening formicaria in its browser…';
    } catch (e) {
      said = e instanceof Error ? e.message : String(e);
    }
  }
</script>

{#if st}
  <section>
    <h3>formicaria’s browser</h3>
    <ManualLink page="user/browser.html" />
    <p class="muted lede">
      Browse the web inside formicaria and keep what you find — a recipe, a paper, an article, a
      video or a song — as a note, with a copy of the page, in one press.
    </p>
    {#if !st.supported}
      <p class="muted">
        Not available on this computer yet: it is set up on Linux first, where it was tried. Your
        notes are unaffected.
      </p>
    {:else if !st.installed}
      <p class="muted">
        It is built on <strong>Firefox ESR</strong>, Mozilla's long-term-support engine, downloaded
        from Mozilla and checked against Mozilla's signature, then made formicaria's: its name, its
        look, and formicaria always one tab away. About <strong>90 MB</strong> to download and
        <strong>320 MB</strong> on this computer, in formicaria's settings folder. Firefox add-ons work
        in it, and you can sign in to your Mozilla account to bring your bookmarks and passwords. Mozilla's
        telemetry, studies, Pocket, sponsored content and AI chatbot are off; your history stays on this
        computer.
      </p>
      {#if st.busy}
        <p>
          {st.phase || 'Working…'}
          {#if st.total}
            <progress max={st.total} value={st.done}></progress>
            <span class="muted small">{mb(st.done)} of {mb(st.total)}</span>
          {/if}
        </p>
      {:else}
        <p><button class="btn" onclick={setUp}>Set up formicaria’s browser</button></p>
      {/if}
    {:else}
      <ul class="caps">
        <li>
          <span class="k">browser</span>
          <code>Firefox {st.version}</code>
          <button class="btn" onclick={openNow}>Open formicaria in it</button>
        </li>
        <li>
          <label class="switch-line">
            <input
              type="checkbox"
              role="switch"
              checked={st.open_in_it}
              onchange={(e) => set({ open_in_it: (e.currentTarget as HTMLInputElement).checked })}
            />
            <span>Open formicaria in its own browser when it starts</span>
          </label>
        </li>
        <li>
          <span class="k">updates</span>
          <span class="muted"
            >security releases arrive by themselves{st.update_ready
              ? ' — one is ready and goes in when the browser next starts'
              : ''}; tested with Firefox {st.pinned}</span
          >
        </li>
        <li>
          <label class="switch-line">
            <input
              type="checkbox"
              role="switch"
              checked={st.block_ads}
              onchange={(e) => set({ block_ads: (e.currentTarget as HTMLInputElement).checked })}
            />
            <span
              >Install uBlock Origin to block ads (Firefox’s own tracking protection is on either
              way)</span
            >
          </label>
        </li>
        <li>
          <label class="switch-line">
            <input
              type="checkbox"
              role="switch"
              checked={st.web_push ?? false}
              onchange={(e) => set({ web_push: (e.currentTarget as HTMLInputElement).checked })}
            />
            <span>Let sites send notifications while their tab is closed</span>
          </label>
        </li>
      </ul>
      <p class="muted small">
        formicaria is the pinned first tab; the formicaria button (or Alt+Shift+F) brings it
        forward. Right-click a page to <em>Keep</em> or <em>Bookmark</em> it in formicaria. Changes take
        effect the next time the browser starts.
      </p>
    {/if}
    {#if st.error}<p class="none">{st.error}</p>{/if}
    {#if said}<p class="muted">{said}</p>{/if}
  </section>
{/if}

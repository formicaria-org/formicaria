<script lang="ts">
  // **Privacy** (`decisions.md` 2026-11-17): each protection a switch, easy on and off, like
  // DuckDuckGo's; what it did on the page in front; a site's protections off when it breaks; the fire
  // button. The same sheet in ≡ → Privacy and in Settings → Privacy, on the phone and the computer.
  import { onMount } from 'svelte';
  import { privacy, privacyError, changePrivacy, loadPrivacy } from './privacy.svelte';
  import type { PrivacyChange } from './ipc';

  let {
    site = '',
    counts = null,
    computer = false,
    vpn = false,
    askFire = false,
  }: {
    /** The site of the web page in front, when there is one. */
    site?: string;
    /** What the protections did on that page (the phone's page view counts). */
    counts?: { blocked?: number; cleaned?: number; upgraded?: number; banners?: number } | null;
    /** On a computer the browser is Firefox: some switches map onto its own protections. */
    computer?: boolean;
    /** The phone build can block trackers in other apps (the F-Droid build). */
    vpn?: boolean;
    /** Opened from 🔥 (the phone's ≡): the question to forget comes first, asked at once. */
    askFire?: boolean;
  } = $props();

  const p = $derived(privacy());
  // Read afresh each time the sheet opens (2026-11-28): a change made elsewhere since (the computer's
  // tab-row 🔥, another tab's sheet) shows, and the question names the sites kept as they are now.
  onMount(() => void loadPrivacy());
  const host = $derived(
    site
      .toLowerCase()
      .replace(/^[a-z]+:\/\//, '')
      .split(/[/?#:]/)[0]
      .replace(/^www\./, ''),
  );
  const off = $derived(!!host && !!p?.sites_off.includes(host));

  type Key =
    'trackers' | 'ads' | 'cookies' | 'clean_links' | 'gpc' | 'https_always' | 'app_trackers';
  const SWITCHES: { key: Key; name: string; line: string; computerLine?: string }[] = [
    {
      key: 'trackers',
      name: 'Block trackers',
      line: 'Requests to companies that follow you from site to site are refused.',
      computerLine: "Firefox's strict tracking protection.",
    },
    {
      key: 'ads',
      name: 'Block ads',
      line: 'Ads are refused, and the empty boxes they leave are hidden. Uses more memory.',
      computerLine: 'uBlock Origin Lite, installed in formicaria’s browser while this is on.',
    },
    {
      key: 'cookies',
      name: 'Refuse cookie pop-ups',
      line: 'Consent banners are answered “reject” for you, and closed.',
      computerLine: "Firefox's own cookie banner reduction.",
    },
    {
      key: 'clean_links',
      name: 'Clean tracking links',
      line: 'Tracking parts (utm_…, fbclid…) are taken off addresses before a page opens.',
      computerLine: "Firefox's own stripping of tracking parts.",
    },
    {
      key: 'gpc',
      name: 'Tell sites not to sell or share',
      line: 'Global Privacy Control: a signal sites must honour in some places (California, the EU).',
    },
    {
      key: 'https_always',
      name: 'Always https',
      line: 'Every page is tried over https first anyway. On: a site with no secure version asks before it opens; off, it opens marked not secure, as in Firefox.',
    },
  ];

  const set = (change: PrivacyChange) => void changePrivacy(change);

  // **Trackers in other apps** (the phone's `TrackerVpn`): which app tried which trackers, read while
  // the sheet is open; an app let through, or refused again. Android's permission refused: off again.
  interface VpnApp {
    pkg: string;
    label: string;
    n: number;
    hosts: { host: string; n: number }[];
  }
  const vpnShell = () =>
    (
      globalThis as {
        __fmVpn?: { state(): string; allow(pkg: string, through: boolean): void };
      }
    ).__fmVpn;
  let vpnState = $state<{
    on: boolean;
    apps: VpnApp[];
    allowed: string[];
    /** The app lists are read (until then every name is answered). */
    ready?: boolean;
    /** Android's Private DNS set to this provider: lookups go there, not through the block. */
    private_dns?: string | null;
  } | null>(null);
  let vpnRefused = $state(false);
  $effect(() => {
    if (computer || !p?.app_trackers || !vpnShell()) return;
    const read = () => {
      try {
        vpnState = JSON.parse(vpnShell()!.state());
      } catch {
        vpnState = null;
      }
    };
    read();
    const t = setInterval(read, 2000);
    return () => clearInterval(t);
  });
  $effect(() => {
    const refused = () => {
      vpnRefused = true;
      set({ app_trackers: false });
    };
    window.addEventListener('fm-vpn', refused);
    return () => window.removeEventListener('fm-vpn', refused);
  });
  function letThrough(pkg: string, through: boolean) {
    vpnShell()?.allow(pkg, through);
    if (vpnState)
      vpnState = {
        ...vpnState,
        allowed: through ? [...vpnState.allowed, pkg] : vpnState.allowed.filter((x) => x !== pkg),
      };
  }
  // svelte-ignore state_referenced_locally
  let confirmFire = $state(askFire);
  const kept = $derived(p?.fireproof ?? []);
  const keptHere = $derived(!!host && kept.includes(host));
  let keepTyped = $state('');
  let keepError = $state('');
  async function keepSite(e: SubmitEvent) {
    e.preventDefault();
    const site = keepTyped.trim();
    if (!site) return;
    keepError = '';
    await changePrivacy({ fireproof_add: site });
    if (privacyError()) keepError = 'That is not a site’s name.';
    else keepTyped = '';
  }
</script>

{#snippet fireBlock()}
  <div class="fire" aria-label="forget web data">
    {#if confirmFire}
      <p>
        Close the web’s tabs and forget every web page’s cookies, stored data, cache and history on
        this device?{kept.length ? ` Kept signed in: ${kept.join(', ')}.` : ''} Your notes and the files
        you kept stay.
      </p>
      <button
        type="button"
        class="binding danger"
        onclick={() => {
          confirmFire = false;
          set({ fire: true });
        }}>Forget them</button
      >
      <button type="button" class="binding" onclick={() => (confirmFire = false)}>Keep them</button>
    {:else}
      <button type="button" class="binding" onclick={() => (confirmFire = true)}
        >🔥 Forget web data</button
      >
      {#if p?.fired}<span class="muted"
          >Forgotten {p.fired === 1 ? 'once' : `${p.fired} times`}.</span
        >{/if}
    {/if}
  </div>
{/snippet}

<section class="privacy" aria-label="privacy">
  {#if !p}
    <p class="muted">{privacyError() || 'Reading this device’s settings…'}</p>
  {:else}
    {#if askFire}{@render fireBlock()}{/if}
    {#if host}
      <div class="here">
        <p class="site">
          <strong>{host}</strong>
          {#if off}
            <span class="muted">— protections off for this site</span>
          {/if}
        </p>
        {#if counts && !off}
          <p class="muted">
            {counts.blocked ?? 0} blocked · {counts.banners ?? 0} pop-ups refused · {counts.cleaned ??
              0}
            links cleaned · {counts.upgraded ?? 0} made secure
          </p>
        {/if}
        <button
          type="button"
          class="binding"
          onclick={() => set(off ? { site_on: host } : { site_off: host })}
        >
          {off ? 'Protect this site again' : 'Protections off for this site'}
        </button>
        <button
          type="button"
          class="binding"
          onclick={() => set(keptHere ? { fireproof_remove: host } : { fireproof_add: host })}
        >
          {keptHere ? 'Stop keeping it signed in' : 'Keep signed in when forgetting'}
        </button>
      </div>
    {/if}

    <label class="row">
      <span class="text">
        <span class="name">Search with</span>
        <span class="muted">The web field searches here. All of these are private.</span>
      </span>
      <select
        value={p.search}
        onchange={(e) => set({ search: (e.currentTarget as HTMLSelectElement).value })}
      >
        {#each p.engines as e (e.key)}
          <option value={e.key}>{e.name}</option>
        {/each}
      </select>
    </label>

    <!-- **Where address lookups go** (the owner, 2026-11-19: encrypted, on by default). -->
    <label class="row">
      <span class="text">
        <span class="name">Encrypted lookups</span>
        <span class="muted"
          >{computer
            ? 'Which sites you open is hidden from your network; this service sees the names instead.'
            : 'With Block trackers in other apps on, the phone’s lookups are hidden from your network; this service sees the names instead.'}</span
        >
      </span>
      <select
        aria-label="Encrypted lookups"
        value={p.dns}
        onchange={(e) => set({ dns: (e.currentTarget as HTMLSelectElement).value })}
      >
        {#each p.resolvers ?? [] as r (r.key)}
          <option value={r.key}>{r.name}</option>
        {/each}
      </select>
    </label>

    {#each SWITCHES as s (s.key)}
      <label class="row">
        <span class="text">
          <span class="name">{s.name}</span>
          <span class="muted">{computer && s.computerLine ? s.computerLine : s.line}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          aria-label={s.name}
          checked={p[s.key]}
          onchange={(e) => set({ [s.key]: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
    {/each}

    {#if computer}
      <label class="row">
        <span class="text">
          <span class="name">More sites</span>
          <span class="muted"
            >Cookie pop-ups Firefox's own rules miss: DuckDuckGo's rules, run by formicaria's own
            add-on, no extension to install.</span
          >
        </span>
        <input
          type="checkbox"
          role="switch"
          aria-label="More sites"
          checked={p.more_sites}
          onchange={(e) => set({ more_sites: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
      <label class="row">
        <span class="text">
          <span class="name">DuckDuckGo's protections too</span>
          <span class="muted"
            >DuckDuckGo Privacy Essentials, its own tracker list on top of Firefox's. Uses more
            memory: formicaria's browser measured about a fifth more with it.</span
          >
        </span>
        <input
          type="checkbox"
          role="switch"
          aria-label="DuckDuckGo's protections too"
          checked={p.ddg}
          onchange={(e) => set({ ddg: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
    {/if}
    {#if !computer}
      <label class="row">
        <span class="text">
          <span class="name">Block trackers in other apps</span>
          <span class="muted">
            {#if vpn}
              The whole phone: a local VPN refuses trackers' addresses, from lists made for apps.
              While on, another VPN cannot run.
            {:else}
              Not in this build of formicaria: update it.
            {/if}
          </span>
        </span>
        <input
          type="checkbox"
          role="switch"
          aria-label="Block trackers in other apps"
          disabled={!vpn}
          checked={p.app_trackers}
          onchange={(e) => {
            vpnRefused = false;
            set({ app_trackers: (e.currentTarget as HTMLInputElement).checked });
          }}
        />
      </label>
      {#if vpnRefused}
        <p class="muted">Android's permission was not given, so the block is off.</p>
      {/if}
      {#if p.app_trackers && vpnState?.private_dns}
        <p class="warn" role="alert">
          Android's Private DNS is set to <b>{vpnState.private_dns}</b>, so other apps look
          addresses up there and the block cannot see them. To use the block, set Android's Settings
          → Network → Private DNS to Automatic or Off.
        </p>
      {/if}
      {#if p.app_trackers}
        <label class="row">
          <span class="text">
            <span class="name">Stop apps going around the block</span>
            <span class="muted"
              >Refuses the public encrypted-lookup services some apps use on their own, so their
              lookups come through the block. An app that insists on its own can stop working: let
              it through below.</span
            >
          </span>
          <input
            type="checkbox"
            role="switch"
            aria-label="Stop apps going around the block"
            checked={p.around}
            onchange={(e) => set({ around: (e.currentTarget as HTMLInputElement).checked })}
          />
        </label>
      {/if}
      {#if p.app_trackers && vpnState}
        <div class="apps" aria-label="apps and their trackers">
          {#if !vpnState.apps.length}
            <p class="muted">No app has tried a tracker yet.</p>
          {/if}
          {#each vpnState.apps as a (a.pkg)}
            <details>
              <summary
                >{a.label} — {a.n} refused{vpnState.allowed.includes(a.pkg)
                  ? ' (let through)'
                  : ''}</summary
              >
              <ul>
                {#each a.hosts as h (h.host)}<li>
                    {h.host} <span class="muted">×{h.n}</span>
                  </li>{/each}
              </ul>
              <button
                type="button"
                class="binding"
                onclick={() => letThrough(a.pkg, !vpnState!.allowed.includes(a.pkg))}
                >{vpnState.allowed.includes(a.pkg)
                  ? 'Refuse its trackers again'
                  : 'Let it through'}</button
              >
            </details>
          {/each}
        </div>
      {/if}
    {/if}

    {#if p.sites_off.length}
      <div class="sites">
        <p class="name">Sites with protections off</p>
        <ul>
          {#each p.sites_off as s (s)}
            <li>
              {s}
              <button
                type="button"
                class="binding"
                aria-label={`protect ${s} again`}
                onclick={() => set({ site_on: s })}>Protect again</button
              >
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if !askFire}{@render fireBlock()}{/if}

    <label class="row">
      <span class="text">
        <span class="name">Forget when closing</span>
        <span class="muted"
          >{computer
            ? 'When formicaria’s browser closes: the web’s tabs and data, except the sites kept signed in.'
            : 'When formicaria starts again, or comes back after five minutes away: the web’s tabs and data, except the sites kept signed in.'}</span
        >
      </span>
      <input
        type="checkbox"
        role="switch"
        aria-label="Forget when closing"
        checked={!!p.fire_on_close}
        onchange={(e) => set({ fire_on_close: (e.currentTarget as HTMLInputElement).checked })}
      />
    </label>

    <div class="sites kept" aria-label="kept signed in">
      <p class="name">Kept signed in</p>
      <p class="muted">
        These sites keep their sign-in when the web’s data is forgotten. Their history and cache
        still go.
      </p>
      {#if kept.length}
        <ul>
          {#each kept as s (s)}
            <li>
              {s}
              <button
                type="button"
                class="binding"
                aria-label={`stop keeping ${s} signed in`}
                onclick={() => set({ fireproof_remove: s })}>Stop keeping</button
              >
            </li>
          {/each}
        </ul>
      {/if}
      <form class="keep" onsubmit={keepSite}>
        <input
          type="text"
          inputmode="url"
          autocapitalize="off"
          placeholder="example.org"
          aria-label="a site to keep signed in"
          bind:value={keepTyped}
        />
        <button type="submit" class="binding">Keep</button>
      </form>
      {#if keepError}<p class="muted" role="alert">{keepError}</p>{/if}
    </div>
  {/if}
</section>

<style>
  .privacy {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.35rem 0;
    border-bottom: 1px solid var(--border, #8883);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .name {
    font-weight: 600;
  }
  .muted {
    opacity: 0.7;
    font-size: 0.9em;
  }
  .warn {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-left: 3px solid var(--accent);
    font-size: 0.9em;
  }
  input[type='checkbox'] {
    width: 1.3rem;
    height: 1.3rem;
    flex: none;
  }
  .here {
    padding: 0.5rem 0.6rem;
    border-radius: 6px;
    background: var(--panel, #8881);
  }
  .here p {
    margin: 0 0 0.3rem;
  }
  .sites ul {
    list-style: none;
    padding: 0;
    margin: 0.3rem 0 0;
  }
  .sites li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.2rem 0;
  }
  .fire p {
    margin: 0 0 0.4rem;
  }
  .keep {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.4rem;
  }
  .keep input {
    flex: 1;
    min-width: 0;
  }
  .danger {
    color: var(--danger, #c33);
  }
</style>

<script lang="ts">
  // **Privacy** (`decisions.md` 2026-11-17): each protection a switch, easy on and off, like
  // DuckDuckGo's; what it did on the page in front; a site's protections off when it breaks; the fire
  // button. The same sheet in ≡ → Privacy and in Settings → Privacy, on the phone and the computer.
  import { privacy, privacyError, changePrivacy } from './privacy.svelte';
  import type { PrivacyChange } from './ipc';

  let {
    site = '',
    counts = null,
    computer = false,
    vpn = false,
  }: {
    /** The site of the web page in front, when there is one. */
    site?: string;
    /** What the protections did on that page (the phone's page view counts). */
    counts?: { blocked?: number; cleaned?: number; upgraded?: number; banners?: number } | null;
    /** On a computer the browser is Firefox: some switches map onto its own protections. */
    computer?: boolean;
    /** The phone build can block trackers in other apps (the F-Droid build). */
    vpn?: boolean;
  } = $props();

  const p = $derived(privacy());
  const host = $derived(
    site
      .toLowerCase()
      .replace(/^[a-z]+:\/\//, '')
      .split(/[/?#:]/)[0]
      .replace(/^www\./, ''),
  );
  const off = $derived(!!host && !!p?.sites_off.includes(host));

  type Key = 'trackers' | 'ads' | 'cookies' | 'clean_links' | 'gpc' | 'https_only' | 'app_trackers';
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
      computerLine: 'uBlock Origin, installed in formicaria’s browser while this is on.',
    },
    {
      key: 'cookies',
      name: 'Refuse cookie pop-ups',
      line: 'Consent banners are answered “reject” for you, and closed.',
      computerLine: "Firefox's cookie banner reduction, with Consent-O-Matic.",
    },
    {
      key: 'clean_links',
      name: 'Clean tracking links',
      line: 'Tracking parts (utm_…, fbclid…) are taken off addresses before a page opens.',
      computerLine: "Firefox's own stripping, with ClearURLs.",
    },
    {
      key: 'gpc',
      name: 'Tell sites not to sell or share',
      line: 'Global Privacy Control: a signal sites must honour in some places (California, the EU).',
    },
    {
      key: 'https_only',
      name: 'Secure connections only',
      line: 'Pages open over https; a page that has no secure version asks first.',
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
  let vpnState = $state<{ on: boolean; apps: VpnApp[]; allowed: string[] } | null>(null);
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
  let confirmFire = $state(false);
</script>

<section class="privacy" aria-label="privacy">
  {#if !p}
    <p class="muted">{privacyError() || 'Reading this device’s settings…'}</p>
  {:else}
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
              The whole phone: a local VPN refuses trackers' addresses. Nothing leaves the phone.
              While on, another VPN cannot run; trackers an app reaches by its own private lookup
              slip by.
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

    <div class="fire">
      {#if confirmFire}
        <p>
          Forget every web page's cookies, stored data, cache and history on this device? Your notes
          and the files you kept stay.
        </p>
        <button
          type="button"
          class="binding danger"
          onclick={() => {
            confirmFire = false;
            set({ fire: true });
          }}>Forget them</button
        >
        <button type="button" class="binding" onclick={() => (confirmFire = false)}
          >Keep them</button
        >
      {:else}
        <button type="button" class="binding" onclick={() => (confirmFire = true)}
          >🔥 Forget web data</button
        >
        {#if p.fired}<span class="muted"
            >Forgotten {p.fired === 1 ? 'once' : `${p.fired} times`}.</span
          >{/if}
      {/if}
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
  .danger {
    color: var(--danger, #c33);
  }
</style>

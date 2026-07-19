<script lang="ts">
  // Settings: what this installation is *actually* configured as.
  //
  // **A mirror, not a form.** Nothing here is editable, and that is a property of the backend
  // rather than a preference: `vaults::save` is append-only and never rewrites an existing
  // entry, so a field offering to change a vault's path or restic repo would silently do
  // nothing — the worst kind of control. Where something genuinely is editable it stays where
  // it already is: the backup panel owns remotes and identity, and NewVault owns creation.
  //
  // What it is for is the question nobody could answer before: *which* vaults, at *which*
  // paths, backing up *where*, under *which* overrides. `VaultInfo.path` has been on the wire
  // for months and was rendered nowhere.
  //
  // Cheap on purpose. `config` shells out to nothing, unlike `backup_status`, which runs
  // `git ls-remote` per vault and is the slowest command in the app. Opening Settings must
  // never be a reason to hit the network.
  import { onMount } from 'svelte';
  import { config as fetchConfig } from './ipc';
  import type { Config } from './types';

  // The one *setting* on this screen. Everything else here is a mirror of configuration the
  // backend owns and this panel cannot change (`vaults::save` is append-only). Layout is
  // different in kind: a view preference like the theme, stored in this browser, touching no
  // vault. That is why it is allowed to live here without making Settings a form.
  let {
    onclose,
    onbackup,
    layout,
    onlayout,
  }: {
    onclose: () => void;
    onbackup: () => void;
    layout: 'auto' | 'tiled' | 'single';
    onlayout: (l: 'auto' | 'tiled' | 'single') => void;
  } = $props();

  let cfg = $state<Config | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      cfg = await fetchConfig();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  const resticFor = (name: string) => cfg?.restic.find((r) => r.vault === name)?.repo ?? null;

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="settings-overlay">
  <button class="settings-backdrop" aria-label="close settings" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="settings">
    <div class="panel-head">
      <h2>Settings</h2>
      <span class="sub">what this installation is configured as</span>
    </div>

    {#if error}
      <p class="bad" role="alert">{error}</p>
    {:else if !cfg}
      <p class="muted">Reading configuration…</p>
    {:else}
      <section>
        <h3>Layout</h3>
        <p class="muted">
          How views are arranged. <strong>Single</strong> is reachable here on any machine on
          purpose — a narrow layout only a phone could run would be a second frontend wearing a
          setting, and nothing would exercise it during ordinary desktop work.
        </p>
        <ul class="caps">
          {#each [['auto', 'Automatic', 'Tiled when there is room, single when there is not.'], ['tiled', 'Tiled', 'The pane grid, always.'], ['single', 'Single', 'One view at a time, with a switcher.']] as [value, label, why] (value)}
            <li>
              <label class="choice">
                <input
                  type="radio"
                  name="layout"
                  checked={layout === value}
                  onchange={() => onlayout(value as 'auto' | 'tiled' | 'single')} />
                <span class="k">{label}</span>
                <span class="muted">{why}</span>
              </label>
            </li>
          {/each}
        </ul>
      </section>

      <section>
        <h3>Vault list</h3>
        {#if cfg.vault_list}
          <p class="path"><code>{cfg.vault_list}</code></p>
          {#if !cfg.vault_list_writable}
            <!-- Not merely "no permission": `vaults::load` also reports unwritable when the
                 file exists and could not be parsed, and in that case we will never overwrite
                 it. Both mean "creating a vault would not survive a restart", which is the bit
                 that matters. -->
            <p class="warn">
              Not writable — either the file can't be written, or it couldn't be parsed and we
              won't overwrite it. New vaults wouldn't survive a restart.
            </p>
          {/if}
        {:else}
          <p class="warn">
            No config directory on this machine, so there is nowhere to save a vault list.
            Vaults come from <code>FM_VAULT</code> only, and nothing can be added.
          </p>
        {/if}
      </section>

      <section>
        <h3>Vaults <span class="count">{cfg.vaults.length}</span></h3>
        {#each cfg.vaults as v (v.name)}
          <div class="vault">
            <div class="line">
              <strong>{v.name}</strong>
              {#if v.default}<span class="tag">default</span>{/if}
            </div>
            <!-- Been on the wire since `list_vaults` existed and rendered nowhere until now.
                 "Which folder is this actually?" had no answer inside the app. -->
            <p class="path"><code>{v.path}</code></p>
            <p class="muted">
              restic:
              {#if resticFor(v.name)}
                <code>{resticFor(v.name)}</code>
              {:else}
                <span class="none">not configured</span> — set <code>restic</code> on this
                vault in the vault list file; there is no UI for it.
              {/if}
            </p>
          </div>
        {/each}
        <p class="muted">
          Remotes and committer identity live in
          <button class="link" onclick={onbackup}>Back up</button>, which is also where they are
          editable. They are not repeated here because reading them runs a network call per
          vault.
        </p>
      </section>

      <section>
        <h3>This machine</h3>
        <ul class="caps">
          <li>
            <span class="k">git</span>
            {#if cfg.git}
              <span class="ok">available</span>
            {:else}
              <span class="none">not installed</span> — notes are still files and still safe;
              there is no history, backup or sharing without it.
            {/if}
          </li>
          <li>
            <span class="k">restic</span>
            {#if cfg.restic_installed}
              <span class="ok">available</span>
            {:else}
              <span class="none">not installed</span> — heavy media has nowhere to back up,
              and a vault cannot be restored from a backup on this machine.
            {/if}
          </li>
          <li>
            <span class="k">restic password</span>
            {#if cfg.restic_password_set}
              <span class="ok">set</span>
            {:else}
              <span class="none">unset</span> — <code>RESTIC_PASSWORD</code> is read from the
              environment and never stored. Heavy backups can't run without it.
            {/if}
          </li>
        </ul>
      </section>

      {#if cfg.env.length}
        <section>
          <h3>Environment overrides</h3>
          <p class="muted">
            In effect right now, and they win over the config file. This is usually the answer
            to "why is it using that vault?".
          </p>
          <ul class="env">
            {#each cfg.env as e (e.name)}
              <li><code>{e.name}</code><span class="eq">=</span><code>{e.value}</code></li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}

    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 8vh;
    z-index: 80;
  }
  .settings-backdrop {
    position: fixed;
    inset: 0;
    padding: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(38rem, 94vw);
    max-height: 82vh;
    overflow-y: auto;
    box-sizing: border-box;
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .panel-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }
  h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-weight: 600;
  }
  .sub,
  .muted {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .muted {
    margin: var(--space-2) 0 0;
    line-height: 1.5;
  }
  section {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }
  .vault {
    padding: var(--space-2) 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .path {
    margin: 2px 0 0;
    font-size: var(--text-sm);
    word-break: break-all;
  }
  code {
    font-size: 0.85em;
  }
  .tag,
  .count {
    font-size: 0.75rem;
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
  .ok {
    color: var(--text);
  }
  .none {
    color: var(--text-muted);
    font-style: italic;
  }
  .warn,
  .bad {
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
    line-height: 1.5;
    color: var(--danger, #c0392b);
  }
  .caps,
  .env {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .k {
    display: inline-block;
    min-width: 9rem;
    color: var(--text-muted);
  }
  .eq {
    color: var(--text-muted);
    padding: 0 4px;
  }
  .choice {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    cursor: pointer;
  }
  .choice .k {
    min-width: 5.5rem;
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: var(--text);
    text-decoration: underline;
    cursor: pointer;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  .actions button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--fg);
    padding: 4px 10px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
</style>

<script lang="ts">
  // Settings: what this installation is *actually* configured as.
  //
  // **A mirror, not a form.** Nothing here is editable, and that stays true now that a vault's
  // restic repo *is* settable (2026-08-29, via `vaults::set_restic` — a narrow writer, because
  // `vaults::save` still refuses to rewrite an existing entry). The reason is placement, not
  // capability: every editable thing lives where it is used, so the backup panel owns remotes,
  // identity, backup repos and the backup password, and NewVault owns creation. This screen
  // answers "what is this installation actually configured as" and points at whoever owns the
  // change — which is why the rows below name a panel rather than a config file.
  //
  // What it is for is the question nobody could answer before: *which* vaults, at *which*
  // paths, backing up *where*, under *which* overrides. `VaultInfo.path` has been on the wire
  // for months and was rendered nowhere.
  //
  // Cheap on purpose. `config` shells out to nothing, unlike `backup_status`, which runs
  // `git ls-remote` per vault and is the slowest command in the app. Opening Settings must
  // never be a reason to hit the network.
  import { onMount } from 'svelte';
  import Appearance from './Appearance.svelte';
  import {
    config as fetchConfig,
    setGitAssetsMax,
    setSupervision,
    agentStatus,
    agentModels,
    removeAgentModel,
    setAgent,
    setTranscribe,
    alive,
    setShare,
    shareCode,
    revokeDevices,
  } from './ipc';
  import { isRemote, shareSummary, type ShareStatus } from './remote';
  import type { Config } from './types';
  import * as keys from './keys';
  import { GIT_ASSETS_CEILING, GIT_ASSETS_WARN, humanSize } from './size';

  let assetError = $state<string | null>(null);

  let consentError = $state<string | null>(null);

  /** Record what may be done with a vault's review record.
   *
   *  Two switches, never one, because agreeing to keep a record of your own corrections is not
   *  agreeing to publish them — collapsing those is exactly what stranded the only comparable open
   *  corpus of AI corrections. Each answer is written on its own; the other is left as it was. */
  async function setConsent(vault: string, next: { collect?: boolean; publish?: boolean }) {
    consentError = null;
    try {
      const vaults = await setSupervision(vault, next);
      if (cfg) cfg = { ...cfg, vaults };
    } catch (e) {
      consentError = e instanceof Error ? e.message : String(e);
    }
  }

  /** Write the vault's attachment limit and take the refreshed list back. The backend parses the
   *  size, so a typo is refused there and reported here rather than being half-applied. */
  async function setAssetMax(vault: string, raw: string) {
    assetError = null;
    try {
      const vaults = await setGitAssetsMax(vault, raw.trim());
      if (cfg) cfg = { ...cfg, vaults };
    } catch (e) {
      assetError = e instanceof Error ? e.message : String(e);
    }
  }

  // The one *setting* on this screen. Everything else here is a mirror of configuration the
  // backend owns and this panel cannot change (`vaults::save` is append-only). Layout is
  // different in kind: a view preference like the theme, stored in this browser, touching no
  // vault. That is why it is allowed to live here without making Settings a form.
  let {
    onclose,
    onbackup,
    layout,
    onlayout,
    columns,
    oncolumns,
    theme,
    ontheme,
    userTheme = null,
    onusertheme = () => {},
    commands,
    section = '',
    onkeyschanged,
  }: {
    onclose: () => void;
    onbackup: () => void;
    layout: 'single' | 'tiled';
    onlayout: (l: 'single' | 'tiled') => void;
    /** `auto` tracks the number of open views; a number pins the grid width. */
    columns: number | 'auto';
    oncolumns: (c: number | 'auto') => void;
    theme: string;
    ontheme: () => void;
    /** The user's own theme file, or `null` for the built-in look. Owned and applied by `App`. */
    userTheme?: import('./appearance').Selection | null;
    onusertheme?: (sel: import('./appearance').Selection | null) => void;
    /** The actions that used to be the command palette. Grouped, and run from here.
     *  `command` names the binding this action answers to, when it has one — so the shortcut is
     *  shown next to the thing it does, which is where a person looks for it, rather than only in
     *  the Keyboard section at the bottom of this panel. */
    commands: { label: string; run: () => void; group?: string; command?: keys.Command }[];
    /** Which heading to open at — `commands` when a "+" button sent you here. */
    section?: string;
    /** Told when a binding changes, so the shell re-reads it without a reload. */
    onkeyschanged?: () => void;
  } = $props();

  let cfg = $state<Config | null>(null);
  let error = $state<string | null>(null);

  // **A view preference, like Layout and the theme** — stored in this browser, touching no
  // vault. That is what lets a rebind live on a screen that is otherwise a read-only mirror of
  // configuration the backend owns: this is not vault config, so the rule Settings keeps
  // ("never offer a control that silently does nothing") is not in play.
  let keymap = $state(keys.load());
  let filter = $state('');
  const shown = $derived(
    filter.trim()
      ? commands.filter((c) => c.label.toLowerCase().includes(filter.trim().toLowerCase()))
      : commands,
  );
  let capturing = $state<keys.Command | null>(null);
  const COMMANDS = Object.keys(keys.LABELS) as keys.Command[];

  function capture(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      capturing = null;
      return;
    }
    // A lone modifier is someone mid-chord, not a binding.
    if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return;
    keymap = { ...keymap, [capturing]: keys.fromEvent(e) };
    keys.save(keymap);
    onkeyschanged?.();
    capturing = null;
  }

  function resetKeys() {
    keys.reset();
    keymap = keys.load();
    onkeyschanged?.();
  }

  onMount(async () => {
    // Land on the requested heading, so a "+" button arrives where it meant to rather than at
    // the top of a long panel.
    if (section) queueMicrotask(() => document.getElementById(section)?.scrollIntoView());
    try {
      cfg = await fetchConfig();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    // The study-assistant toggle. `null` = unavailable here (e.g. a build without the agent), which
    // hides the row rather than showing a control that does nothing.
    try {
      const st = await agentStatus();
      agentOn = st.enabled;
      transcribeOn = st.transcribe;
      agentInstalled = st.installed;
      provisioned = st.provisioned;
      provisionedBytes = st.provisioned_bytes;
      provisioning = st.provisioning;
      if (
        st.provisioning &&
        st.provisioning.stage !== 'ready' &&
        st.provisioning.stage !== 'failed'
      ) {
        watchProvisioning();
      }
      agentWhy = st.why;
      transcribeAvailable = st.transcribe_available;
      transcribeFetchable = st.transcribe_fetchable;
    } catch {
      agentOn = null;
    }
    // Sharing: the setting and the capability, read together at open.
    await refreshShare();
  });

  // The local study assistant: whether it auto-starts with formicaria. Off by default; a change
  // takes effect at the next launch. `null` while unknown/unavailable.
  let agentOn = $state<boolean | null>(null);
  // **Whether the assistant can actually run here** — not whether the switch is on. Separate from
  // `agentOn` on purpose: a stored preference cannot fail, so conflating them is how this panel came
  // to offer a switch that reported success and did nothing.
  let agentInstalled = $state(true);
  // **Why not**, in the server's words. The causes are different — this OS has no resource monitor
  // yet, the stack was never shipped to this machine, `bash` is missing — and each wants a different
  // answer from the reader, so the row prints the reason rather than a bare "not available".
  let agentWhy = $state('');
  // Audio transcription (whisper) — a sub-setting of the assistant. Persisted like the on/off above;
  // it loads a local speech-to-text runtime the assistant uses for `/transcribe`.
  let transcribeOn = $state(false);
  // **Its own capability, not the assistant's.** Whisper is a separate runtime, so the assistant can
  // be fully working while this has nothing behind it — which is how the toggle came to store a
  // preference, report success, and transcribe nothing.
  let transcribeAvailable = $state(true);
  let transcribeFetchable = $state(false);
  async function toggleTranscribe(next: boolean) {
    try {
      await setTranscribe(next);
      transcribeOn = next;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  // **The first enable is a question, not a switch.** Turning the assistant on for the first time
  // downloads a model measured in gigabytes; a toggle that starts that silently is the failure the
  // phone already has, where the switch flips and nothing visibly happens for minutes. So: offer
  // the catalogue with sizes and licences, then download with a progress line and a cancel.
  let choosing = $state(false);
  let catalogue = $state<Awaited<ReturnType<typeof agentModels>>>([]);
  let pickedModel = $state<string | null>(null);
  let pickVision = $state(false);
  let provisioned = $state(true);
  let provisionedBytes = $state(0);
  /// Armed by the first click, acted on by the second — the two-step `BackupPanel` already uses for
  /// "forget this vault", and for the same reason: this frees gigabytes and cannot be undone.
  let removingModel = $state(false);
  let removedNote = $state<string | null>(null);
  let provisioning = $state<Awaited<ReturnType<typeof agentStatus>>['provisioning']>(null);
  let pollTimer: ReturnType<typeof setTimeout> | undefined;

  const picked = $derived(catalogue.find((m) => m.name === pickedModel) ?? null);
  /** What this choice will actually download, projector included when it is wanted. */
  const pickedBytes = $derived(
    (picked?.bytes ?? 0) + (pickVision && picked?.vision ? (picked?.mmproj_bytes ?? 0) : 0),
  );

  async function openChooser() {
    error = null;
    try {
      catalogue = await agentModels();
      pickedModel = catalogue.find((m) => m.default)?.name ?? catalogue[0]?.name ?? null;
      pickVision = false;
      choosing = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Poll while a download runs. The app has no streaming anywhere; this is the cadence the
   *  agent's own working-wheel already uses, for the same reason. */
  function watchProvisioning() {
    clearTimeout(pollTimer);
    pollTimer = setTimeout(async () => {
      try {
        const st = await agentStatus();
        provisioning = st.provisioning;
        provisioned = st.provisioned;
        if (
          st.provisioning &&
          st.provisioning.stage !== 'ready' &&
          st.provisioning.stage !== 'failed'
        ) {
          watchProvisioning();
        }
      } catch {
        // A failed poll is not a failed download — keep watching rather than reporting a fault
        // that may not exist.
        watchProvisioning();
      }
    }, 1500);
  }

  async function toggleAgent(next: boolean) {
    // Off, or already provisioned: the switch it always was.
    if (!next || provisioned) {
      try {
        await setAgent(next);
        agentOn = next;
        if (!next) provisioning = null;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
      return;
    }
    await openChooser();
  }

  /** Start the download the chooser described. */
  async function startProvisioning() {
    choosing = false;
    try {
      await setAgent(true, pickedModel ?? undefined, pickVision);
      agentOn = true;
      provisioning = { stage: 'model', done: 0, total: null, error: null };
      watchProvisioning();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Delete the downloaded model and reclaim the disk. Turns the assistant off as a side effect,
   *  which the server does too — a model cannot be removed while it is running. */
  async function removeModel() {
    try {
      const { freed } = await removeAgentModel();
      removingModel = false;
      provisioned = false;
      provisionedBytes = 0;
      agentOn = false;
      removedNote = `Removed. ${humanSize(freed)} is free again.`;
    } catch (e) {
      removingModel = false;
      error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Stop a download in flight. The server bumps its generation, so the worker abandons the work;
   *  what it already fetched stays, and enabling again resumes rather than restarts. */
  async function cancelProvisioning() {
    clearTimeout(pollTimer);
    try {
      await setAgent(false);
      agentOn = false;
      provisioning = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // ---- Sharing with a nearby device ---------------------------------------------------------
  //
  // Two separate pieces of state on purpose, and conflating them is the mistake this feature was
  // most at risk of: `shareOn` is **what the user asked for**, `share` is **what is actually
  // happening**. `restic_ready` is the cautionary tale — it meant "repo set + password set", so
  // on a machine with no restic the checkbox enabled, you ticked it, and the backup failed.
  let shareOn = $state(false);
  let share = $state<ShareStatus | null>(null);
  let shareError = $state('');
  let code = $state('');
  /// Which vaults the *next* code will grant. Starts empty rather than "all", so sharing
  /// everything is something you do on purpose and never by pressing the obvious button.
  let chosen = $state<string[]>([]);
  let codeTimer: ReturnType<typeof setTimeout> | undefined;

  async function refreshShare() {
    // The capability rides the liveness beat — it is the transport's own signal, and a listener
    // can fail long after this panel was last opened.
    share = await alive();
    if (share) shareOn = share.state !== 'off';
  }

  async function toggleShare(next: boolean) {
    shareError = '';
    try {
      await setShare(next);
      shareOn = next;
      // The listener only changes at the next launch, so say so rather than letting the status
      // line below look like it is lagging.
      shareError = next
        ? 'Sharing starts at the next launch — quit and reopen formicaria.'
        : 'Sharing stops at the next launch.';
    } catch (e) {
      shareError = e instanceof Error ? e.message : String(e);
      shareOn = !next;
    }
  }

  async function startCode() {
    shareError = '';
    try {
      const r = await shareCode(chosen);
      code = r.code;
      // Clear it when it dies, so the screen never shows a code that no longer works — someone
      // typing a stale code gets "that code is not right", which reads like their own mistake.
      clearTimeout(codeTimer);
      codeTimer = setTimeout(() => (code = ''), r.expires_in * 1000);
    } catch (e) {
      shareError = e instanceof Error ? e.message : String(e);
    }
  }

  async function revoke() {
    shareError = '';
    try {
      await revokeDevices();
      await refreshShare();
      shareError = 'All devices disconnected.';
    } catch (e) {
      shareError = e instanceof Error ? e.message : String(e);
    }
  }

  const resticFor = (name: string) => cfg?.restic.find((r) => r.vault === name)?.repo ?? null;

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={(e) => (capturing ? capture(e) : onkeydown(e))} />

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
      <!-- **Two kinds of thing on one screen, and the split is the point.** Everything above
           the divider is a *preference*: it lives in this browser, touches no vault, and can be
           changed here. Everything below is a *mirror* of configuration the backend owns, which
           this panel deliberately cannot edit — `vaults::save` is append-only, so a field
           offering to change a vault's path would silently do nothing. -->
      <!-- **Actions first, preferences second.** This is the surface the "+" buttons and the
           gear both land on, and what someone wants nine times out of ten is to open or make
           something — not to change a setting. -->
      <p class="group" id="commands">Do something</p>

      <section>
        <input
          class="filter"
          placeholder="Filter actions…"
          bind:value={filter}
          autocomplete="off"
          spellcheck="false"
        />
        <ul class="action-list">
          {#each shown as c, i (c.label)}
            {#if c.group && c.group !== shown[i - 1]?.group}
              <li class="sub">{c.group}</li>
            {/if}
            <li>
              <button
                class="action"
                onclick={() => {
                  c.run();
                  onclose();
                }}
              >
                <span>{c.label}</span>
                <!-- The standard menu pattern: the accelerator beside the thing it triggers.
                     Rebinding has shipped for a while and was invisible unless you scrolled to the
                     bottom of this panel — Nielsen's seventh heuristic asks for accelerators to be
                     *visible* to the people who would use them. An unbound command says so, which
                     is how `newView` stops being a capability you can only find in the source. -->
                {#if c.command}
                  <span class="binding-chip" class:unbound={!keymap[c.command]?.key}>
                    {keymap[c.command]?.key ? keys.describe(keymap[c.command]) : 'not set'}
                  </span>
                {/if}
              </button>
            </li>
          {/each}
          {#if shown.length === 0}
            <li class="muted">Nothing matches “{filter}”.</li>
          {/if}
        </ul>
      </section>

      <p class="group">Preferences <span class="muted">— this browser, no vault touched</span></p>

      <section>
        <h3>Theme</h3>
        <p class="muted">
          Stored in this browser. It followed the system setting until you chose here.
        </p>
        <ul class="caps">
          <li>
            <span class="k">{theme === 'dark' ? 'Dark' : 'Light'}</span>
            <button class="binding" onclick={ontheme}>
              Switch to {theme === 'dark' ? 'light' : 'dark'}
            </button>
          </li>
        </ul>
      </section>

      <!-- Colours and type. Sits next to the dark/light switch because that is where a person
           looks for "how this looks", even though this one writes a file in the vault while its
           neighbours touch only this browser — which the section says of itself. -->
      <Appearance selected={userTheme} onselect={onusertheme} />

      <section>
        <h3>Layout</h3>
        <p class="muted">
          How views are arranged. <strong>One view</strong> is the default everywhere, including this
          machine — a narrow layout only a phone could run would be a second frontend wearing a setting,
          and nothing would exercise it during ordinary desktop work.
        </p>
        <ul class="caps">
          {#each [['single', 'One view', 'The default. One at a time; the strip along the bottom moves between the ones you have open.'], ['tiled', 'Tiled', 'The pane grid — several views side by side.']] as [value, label, why] (value)}
            <li>
              <label class="choice">
                <input
                  type="radio"
                  name="layout"
                  checked={layout === value}
                  onchange={() => onlayout(value as 'single' | 'tiled')}
                />
                <span class="k">{label}</span>
                <span class="muted">{why}</span>
              </label>
            </li>
          {/each}
        </ul>
      </section>

      {#if agentOn !== null}
        <section>
          <h3>Study assistant</h3>
          <p class="muted">
            A small local model that reads your notes and answers in discussions — mention it by
            name (e.g. <code>@qwen3-vl-4b</code>) and it replies; it can propose edits you review,
            and never touches your notes on its own. It runs entirely on this device, starts and
            stops
            <strong>with formicaria</strong>, and is <strong>off by default</strong>. A change takes
            effect at the next launch.
          </p>
          <ul class="caps">
            {#if !agentInstalled}
              <!-- No switch at all when there is nothing to switch on. The same shape git and restic
                   already use: name what is missing, say what it costs, say the notes are fine.
                   The words come from the server, which is the only thing that knows which of the
                   several reasons applies here. -->
              <li>
                <span class="k">not available</span>
                <span class="muted">{agentWhy}</span>
              </li>
            {:else}
              <li>
                <label class="choice">
                  <!-- Named explicitly: this and the sharing switch both read "Off" from their
                     first span, so without a label a screen reader announces two identical
                     checkboxes on one screen. -->
                  <input
                    type="checkbox"
                    aria-label="study assistant"
                    checked={agentOn}
                    onchange={(e) => toggleAgent(e.currentTarget.checked)}
                  />
                  <span class="k">{agentOn ? 'On' : 'Off'}</span>
                  <span class="muted">
                    {agentOn
                      ? 'Starts with formicaria on the next launch.'
                      : provisioned
                        ? 'Formicaria runs pure and super-light.'
                        : 'Turning it on downloads a model first — it will ask before it does.'}
                  </span>
                </label>
              </li>

              <!-- **Ask, with the numbers, before spending anything.** The size and the licence are
                 read from the catalogue rather than written here, so the figures cannot drift from
                 what is actually fetched. -->
              {#if choosing}
                <li class="vault">
                  <p class="why">
                    The assistant needs a model on this computer. It is downloaded once, kept on
                    this machine, and used offline afterwards. Pick one:
                  </p>
                  {#each catalogue as m (m.name)}
                    <label class="choice">
                      <input
                        type="radio"
                        name="fm-agent-model"
                        value={m.name}
                        checked={pickedModel === m.name}
                        onchange={() => (pickedModel = m.name)}
                      />
                      <span class="k">{m.name}</span>
                      <span class="muted">
                        {m.bytes ? humanSize(m.bytes) : 'size unknown'}{m.license
                          ? ` · ${m.license}`
                          : ''}{m.vision ? ' · can read images' : ''}
                      </span>
                    </label>
                  {/each}
                  {#if picked?.vision}
                    <label class="choice">
                      <input
                        type="checkbox"
                        checked={pickVision}
                        onchange={(e) => (pickVision = e.currentTarget.checked)}
                      />
                      <span class="k">Read images too</span>
                      <span class="muted">
                        A further {picked.mmproj_bytes
                          ? humanSize(picked.mmproj_bytes)
                          : 'download'}
                        — needed to turn a photographed page into text. Without it the model works normally
                        and says it cannot see pictures.
                      </span>
                    </label>
                  {/if}
                  <p class="why">
                    <strong>Total: {humanSize(pickedBytes)}.</strong> You can stop it at any time; what
                    has already arrived is kept, and starting again continues where it left off.
                  </p>
                  <div class="row">
                    <button class="primary" onclick={startProvisioning} disabled={!pickedModel}>
                      Download and turn on
                    </button>
                    <button onclick={() => (choosing = false)}>Cancel</button>
                  </div>
                </li>
              {/if}

              <!-- **Getting the space back.** Nothing in the app could reclaim 2.5-3.3 GB until now,
                 and no document said where the files were. Two steps, like forgetting a vault: this
                 is irreversible and large. It names the figure, because "delete 2.5 GB" is a
                 decision someone can make and "delete the model" is a leap of faith. -->
              {#if provisioned && provisionedBytes > 0 && !provisioning}
                <li>
                  {#if removingModel}
                    <span class="k">remove the model?</span>
                    <span class="muted">
                      Frees {humanSize(provisionedBytes)}. Your notes are untouched — this deletes
                      only the downloaded model and its runtime. Turning the assistant on again asks
                      which model you want and downloads it afresh.
                    </span>
                    <button class="primary" onclick={removeModel}>Yes, remove it</button>
                    <button onclick={() => (removingModel = false)}>Cancel</button>
                  {:else}
                    <span class="k">disk</span>
                    <span class="muted">
                      The model and its runtime use {humanSize(provisionedBytes)}.
                    </span>
                    <button onclick={() => (removingModel = true)}>Remove the model…</button>
                  {/if}
                </li>
              {/if}
              {#if removedNote}
                <li><span class="muted">{removedNote}</span></li>
              {/if}

              <!-- What it is doing now. Bytes, not a percentage, when the server sends no length —
                 a made-up percentage is worse than an honest number. -->
              {#if provisioning && provisioning.stage !== 'ready'}
                <li>
                  {#if provisioning.stage === 'failed'}
                    <span class="k">download failed</span>
                    <span class="muted">{provisioning.error ?? 'no reason given'}</span>
                  {:else}
                    <span class="k">
                      {provisioning.stage === 'runtime'
                        ? 'getting the runtime'
                        : provisioning.stage === 'projector'
                          ? 'getting the image reader'
                          : 'downloading the model'}
                    </span>
                    <span class="muted">
                      {humanSize(provisioning.done)}{provisioning.total
                        ? ` of ${humanSize(provisioning.total)}`
                        : ''} — you can keep working; it continues in the background.
                    </span>
                    <button onclick={cancelProvisioning}>Stop</button>
                  {/if}
                </li>
              {/if}
            {/if}
            {#if agentInstalled && agentOn}
              {#if transcribeAvailable}
                <li>
                  <label class="choice">
                    <input
                      type="checkbox"
                      checked={transcribeOn}
                      onchange={(e) => toggleTranscribe(e.currentTarget.checked)}
                    />
                    <span class="k">Audio transcription {transcribeOn ? 'on' : 'off'}</span>
                    <span class="muted">
                      {transcribeOn
                        ? 'The assistant transcribes audio clips you record or attach (record → /transcribe → a proposal). Applies at the next assistant start.'
                        : 'Turn on to let the assistant transcribe audio into notes.'}
                    </span>
                  </label>
                </li>
              {:else}
                <!-- Speech-to-text is a second runtime, downloaded separately, and the assistant
                     starts it only when it is there. So this stops being a switch when it would be
                     a switch onto nothing — the same rule as the section above it. -->
                <li>
                  <!-- **Two different "no", and only one of them is the user's problem.** Not
                       downloaded yet is an offer; no build published for this kind of computer is
                       a fact they cannot act on, and saying "not on this machine" for that would
                       send someone hunting for a file that does not exist. -->
                  {#if transcribeFetchable}
                    <label class="choice">
                      <input
                        type="checkbox"
                        aria-label="audio transcription"
                        checked={transcribeOn}
                        onchange={(e) => toggleTranscribe(e.currentTarget.checked)}
                      />
                      <span class="k">Audio transcription</span>
                      <span class="muted">
                        Turns recordings into text. Needs a further download of about 170 MB, which
                        it fetches when you turn this on; it starts with the assistant next time.
                      </span>
                    </label>
                  {:else}
                    <span class="k">Audio transcription unavailable</span>
                    <span class="muted">
                      No speech-to-text runtime has been published for this kind of computer yet, so
                      recordings cannot be transcribed here. The assistant works normally without
                      it.
                    </span>
                  {/if}
                </li>
              {/if}
            {/if}
          </ul>
        </section>
      {/if}

      <!-- Shown only on the computer doing the sharing. A paired tablet is refused every route
           behind this panel anyway, so rendering it there would be a menu of failures. -->
      {#if !isRemote()}
        <section>
          <h3>Share with another device</h3>
          <p class="muted">
            Open your notes on a tablet or phone on the same network — useful for touch and for
            taking photos straight into a note. The notes stay <strong>on this computer</strong>;
            the other device reads and writes them over your own network, and nothing is sent
            anywhere else. <strong>Off by default</strong>, and a change takes effect at the next
            launch.
          </p>
          <ul class="caps">
            <li>
              <label class="choice">
                <input
                  type="checkbox"
                  checked={shareOn}
                  onchange={(e) => toggleShare(e.currentTarget.checked)}
                />
                <span class="k">{shareOn ? 'On' : 'Off'}</span>
                <!-- **The capability, not the setting.** "Listening" is not "working": a bound
                     socket says nothing about whether the wifi carries device-to-device traffic
                     or a firewall is dropping the port. `shareSummary` says which. -->
                <span class="muted">{share ? shareSummary(share) : 'Checking…'}</span>
              </label>
            </li>

            {#if shareOn}
              <li>
                <span class="k">Which vaults</span>
                <span class="muted">
                  A device is let into the vaults you choose here and <strong>no others</strong> — it
                  cannot see, search or open anything in the rest.
                </span>
                <ul class="caps">
                  {#each cfg?.vaults ?? [] as v (v.name)}
                    <li>
                      <label class="choice">
                        <input
                          type="checkbox"
                          checked={chosen.includes(v.name)}
                          onchange={(e) =>
                            (chosen = e.currentTarget.checked
                              ? [...chosen, v.name]
                              : chosen.filter((n) => n !== v.name))}
                        />
                        <span class="k">{v.name}</span>
                      </label>
                    </li>
                  {/each}
                </ul>
              </li>

              <!-- The certificate step, and it comes *before* the code deliberately: on a TLS
                   listener the device cannot load the page at all until it trusts the
                   certificate, so offering a pairing code first would send someone to type a
                   code into a browser error. -->
              {#if share?.state === 'listening' && share.fingerprint}
                <li>
                  <span class="k">First, trust this computer</span>
                  <span class="muted">
                    Copy this file to the device and open it, then allow the certificate (on iPad
                    also: <strong>Settings → General → About → Certificate Trust Settings</strong>).
                    Encryption is what lets the device use its microphone.
                  </span>
                  <span class="muted"><code>{share.cert_path}</code></span>
                  <span class="muted">
                    Before you tap install, check the device shows this exact fingerprint. If it
                    shows anything else, <strong>stop</strong> — you are not talking to this computer.
                  </span>
                  <code class="code fp">{share.fingerprint}</code>
                </li>
              {/if}

              <li>
                {#if code}
                  <span class="k">Code: <code class="code">{code}</code></span>
                  <span class="muted">
                    Type this on the other device. It works once, and only for the next few minutes. {#if share?.state === 'listening'}Open
                      <code>{share.url}</code> there first.{/if}
                  </span>
                {:else}
                  <button onclick={startCode} disabled={chosen.length === 0}>
                    Start a pairing code
                  </button>
                  <span class="muted">
                    {chosen.length === 0
                      ? 'Choose at least one vault first.'
                      : 'Shows a short code to type on the other device.'}
                  </span>
                {/if}
              </li>

              {#if (share?.state === 'listening' ? share.devices : 0) > 0}
                <li>
                  <button onclick={revoke}>Disconnect all devices</button>
                  <span class="muted">
                    They will need a new code to connect again. Use this if a device is lost.
                  </span>
                </li>
              {/if}
            {/if}
            {#if shareError}
              <li><span class="muted err">{shareError}</span></li>
            {/if}
          </ul>
        </section>
      {/if}

      <section>
        <h3>Columns</h3>
        <p class="muted">
          How wide the grid is in the tiled arrangement. <strong>Automatic</strong> follows the number
          of open views, so opening one widens the grid and closing one lets the rest reclaim the space.
        </p>
        <ul class="caps">
          {#each ['auto', 1, 2, 3, 4] as c (c)}
            <li>
              <label class="choice">
                <input
                  type="radio"
                  name="columns"
                  checked={columns === c}
                  onchange={() => oncolumns(c as number | 'auto')}
                />
                <span class="k">{c === 'auto' ? 'Automatic' : `${c}`}</span>
              </label>
            </li>
          {/each}
        </ul>
      </section>

      <section>
        <h3>Keyboard</h3>
        <p class="muted">
          Click a shortcut, then press the keys you want. <strong>Esc</strong> cancels. The
          <em>Next / Previous view</em> pair works even while you are typing in a note — which is the
          whole reason they exist, since otherwise a note has to be closed before anything else can be
          reached.
        </p>
        <ul class="caps">
          {#each COMMANDS as cmd (cmd)}
            {@const clash = keys.conflict(keymap, cmd, keymap[cmd])}
            <li>
              <span class="k">{keys.LABELS[cmd]}</span>
              <button
                class="binding"
                class:capturing={capturing === cmd}
                onclick={() => (capturing = capturing === cmd ? null : cmd)}
              >
                {capturing === cmd ? 'press keys…' : keys.describe(keymap[cmd])}
              </button>
              {#if clash}
                <span class="warn">also {keys.LABELS[clash]}</span>
              {:else if keys.reserved(keymap[cmd])}
                <!-- The browser takes this before the page ever sees it, so the binding would
                     simply not work. Said here rather than discovered by pressing it. -->
                <span class="warn">the browser uses this</span>
              {/if}
            </li>
          {/each}
        </ul>
        <p class="muted">
          <button class="link" onclick={resetKeys}>Reset to defaults</button>
        </p>
      </section>

      <p class="group">
        This installation <span class="muted">— read-only; the backend owns these</span>
      </p>

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
              Not writable — either the file can't be written, or it couldn't be parsed and we won't
              overwrite it. New vaults wouldn't survive a restart.
            </p>
          {/if}
        {:else}
          <p class="warn">
            No config directory on this machine, so there is nowhere to save a vault list. Vaults
            come from <code>FM_VAULT</code> only, and nothing can be added.
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
            <!-- **The one vault setting that is editable here**, because it is the one that
                 decides what a backup carries — and "what does Back up actually send?" is the
                 question this whole panel exists to answer. It is written into the vault's own
                 `vault.json`, so the rule travels with the vault instead of being one browser's
                 opinion about everyone's shared history. -->
            <label class="line assets">
              <span>Send attachments under</span>
              <input
                type="text"
                inputmode="text"
                placeholder="off"
                value={v.git_assets_max ? humanSize(v.git_assets_max) : ''}
                onchange={(e) => setAssetMax(v.name, (e.currentTarget as HTMLInputElement).value)}
                aria-label={`largest attachment to push for ${v.name}`}
              />
            </label>
            <p class="muted small">
              {#if v.git_assets_max}
                Files up to {humanSize(Math.min(v.git_assets_max, GIT_ASSETS_CEILING))} are pushed with
                your notes. Anything larger stays on this device — git history is permanent, so a large
                file committed once is in every clone forever.
              {:else}
                Empty means <strong>notes only</strong> — the default. Attachments stay in the vault
                and travel only via restic. The most you can send this way is
                {humanSize(GIT_ASSETS_CEILING)} per file.
              {/if}
            </p>
            <!-- **The ceiling, said where the number is chosen.** There is no git-lfs here, so an
                 attachment in git is permanent history that every clone pays for again. Two
                 different sentences, because they are two different situations: between the
                 warning line and the ceiling the setting *works* and the cost is worth naming;
                 above the ceiling the app is declining to do what the file asks, and hiding that
                 would be the dishonest half of a clamp. The second is reachable only from a
                 `vault.json` written by a hand, another machine, or a version with no ceiling —
                 the field itself refuses those, through `assetError` below. -->
            {#if v.git_assets_max && v.git_assets_max > GIT_ASSETS_CEILING}
              <p class="warn">
                This vault asks for {humanSize(v.git_assets_max)}, which is more than will ever be
                sent: attachments over {humanSize(GIT_ASSETS_CEILING)} are left out, because most hosts
                refuse a file that size and the push would fail after the commit was made. Edit
                <code>vault.json</code> to agree, or leave the heavy ones to the backup snapshot.
              </p>
            {:else if v.git_assets_max && v.git_assets_max > GIT_ASSETS_WARN}
              <p class="warn">
                That is large for git. Files this size are kept forever, downloaded again by every
                clone, and cannot be taken back without rewriting history other people have already
                pulled — and many hosts warn above {humanSize(GIT_ASSETS_WARN)}. The backup snapshot
                carries attachments of any size without any of that.
              </p>
            {/if}
            <!-- Two questions, deliberately not one. The record is this person's own corrections in
                 their own vault, so keeping it needs no ceremony; publishing it relicenses content
                 and cannot be undone, so it is asked separately and never assumed. Lives in
                 `vault.json` beside the attachment rule, for the same reason: it decides what may
                 leave, so the vault settles it rather than whichever device is loosest. -->
            <label class="line consent">
              <input
                type="checkbox"
                checked={v.supervision.collect}
                onchange={(e) =>
                  setConsent(v.name, { collect: (e.currentTarget as HTMLInputElement).checked })}
              />
              <span>Keep a record of what you change in AI suggestions</span>
            </label>
            <label class="line consent">
              <input
                type="checkbox"
                checked={v.supervision.publish}
                onchange={(e) =>
                  setConsent(v.name, { publish: (e.currentTarget as HTMLInputElement).checked })}
              />
              <span>Allow that record to be shared openly</span>
            </label>
            <p class="muted small">
              {#if v.supervision.collect}
                Kept in this vault, with your notes. Nothing is sent anywhere on its own.
                {#if v.supervision.publish}
                  Records made from now on are marked as shareable — sharing itself is still a
                  separate, deliberate step, and it cannot be taken back once taken.
                {:else}
                  Sharing is off, so nothing here can be published.
                {/if}
              {:else}
                Off — nothing is recorded about what you change.
              {/if}
            </p>
            <p class="muted">
              restic:
              {#if resticFor(v.name)}
                <code>{resticFor(v.name)}</code>
              {:else}
                <span class="none">not configured</span> — its attachments stay on this machine. Set
                a backup repo for it in <strong>Backup</strong>.
              {/if}
            </p>
          </div>
        {/each}
        {#if assetError}
          <p class="assets-error">{assetError}</p>
        {/if}
        {#if consentError}
          <p class="assets-error">{consentError}</p>
        {/if}
        <p class="muted">
          Remotes and committer identity live in
          <button class="link" onclick={onbackup}>Back up</button>, which is also where they are
          editable. They are not repeated here because reading them runs a network call per vault.
        </p>
      </section>

      <section>
        <h3>This machine</h3>
        <ul class="caps">
          <!-- **Which build this is.** First on the list because it is the one fact you need
               before any of the others mean anything, and until now it was nowhere in the app.
               Each release unpacks into its own folder and the notes live inside it, so someone
               who has updated is holding two formicarias with no way to tell them apart from the
               inside — and the one they are running is the one this line names. A plain string:
               no check for a newer one, nothing fetched. -->
          <li>
            <span class="k">version</span>
            <code>{cfg.version}</code>
            {#if cfg.version === 'dev'}
              <span class="muted small">— built from source, not a release</span>
            {/if}
          </li>
          <li>
            <span class="k">git</span>
            {#if cfg.git}
              <span class="ok">available</span>
            {:else}
              <span class="none">not installed</span> — notes are still files and still safe; there is
              no history, backup or sharing without it.
            {/if}
          </li>
          <li>
            <!-- Named for what it does, not for poppler. Until 2026-08-28 this failed in complete
                 silence: a PDF ingested with an empty body and search simply never found it. -->
            <span class="k">search inside PDFs</span>
            {#if cfg.pdf_text}
              <span class="ok">available</span>
            {:else}
              <span class="none">not available</span> — PDFs are still stored, opened and shown; their
              contents just are not searchable on this machine.
            {/if}
          </li>
          <li>
            <span class="k">restic</span>
            {#if cfg.restic_installed}
              <span class="ok">available</span>
            {:else}
              <span class="none">not installed</span> — heavy media has nowhere to back up, and a vault
              cannot be restored from a backup on this machine.
            {/if}
          </li>
          {#if cfg.platform === 'ios'}
            <li>
              <!-- **Permanent, not a first-run dialog, and that is the point.** The thing being
                   described recurs every seven days for as long as the app is installed; a notice
                   you dismiss once would be gone before the first time it mattered. It lives here,
                   beside the other facts about what this machine can and cannot do.

                   `platform === 'ios'` rather than a `sideloaded` flag: every iOS build of
                   formicaria is signed by whoever installed it, because there is no App Store
                   route (`decisions.md#track-m`). The OS is the fact; the route is the policy. -->
              <span class="k">this copy expires</span>
              <span class="none">about 7 days after you installed it</span> — it was signed with
              your own Apple ID, and iOS stops opening it when that signature lapses. Refresh it
              with the same tool you installed it with; SideStore can do that over Wi-Fi.
              <strong>Your notes are not affected</strong> — they stay on the phone, and the app opens
              them again once it is refreshed.
            </li>
          {/if}
          {#if cfg.vault_root !== null}
            <li>
              <!-- True of both phones, so it is gated on the managed root rather than on iOS: an
                   app's private storage is removed when the app is, and `blobs/` is gitignored, so
                   a push does not carry it. -->
              <span class="k">photos and files</span>
              <span class="none">live only on this phone</span> until they reach a backup — a git push
              carries your notes but not their media.
            </li>
          {/if}
          {#if cfg.ca_bundle}
            <li>
              <!-- Only on builds that carry their own OpenSSL (the phone). A desktop uses the
                   system trust store and has nothing to report. -->
              <span class="k">certificates</span>
              {#if /^\d+ certificates$/.test(cfg.ca_bundle)}
                <span class="ok">{cfg.ca_bundle}</span>
              {:else}
                <span class="none">{cfg.ca_bundle}</span> — HTTPS remotes cannot be verified without a
                trust store, which git reports only as "the SSL certificate is invalid".
              {/if}
            </li>
          {/if}
          <li>
            <span class="k">restic password</span>
            {#if cfg.restic_password_set}
              <span class="ok">set</span>
            {:else}
              <span class="none">unset</span> — attachment backups cannot run without it. Set one in
              <strong>Backup</strong>; <code>RESTIC_PASSWORD</code> still wins if you'd rather set it
              in the environment.
            {/if}
          </li>
        </ul>
      </section>

      {#if cfg.env.length}
        <section>
          <h3>Environment overrides</h3>
          <p class="muted">
            In effect right now, and they win over the config file. This is usually the answer to
            "why is it using that vault?".
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
    /* `border-box` + an explicit `100dvh`, so the panel below can simply say
       `max-height: 100%` and inherit the arithmetic instead of restating it.
       Longhands, never the `padding` shorthand: a narrower rule added later would reset all
       four sides and silently discard the insets, which is the mistake `ci/checks.sh`
       already guards against on `.topbar`. */
    box-sizing: border-box;
    height: 100dvh;
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
    padding-bottom: var(--safe-bottom);
    z-index: 80;
  }
  /* The navigation-bar floor, on the overlay so the panel's *box* clears the bar and not just
     its last row. `--bar-floor` rather than a fourth hand-copied `3.25rem`. */
  @media (pointer: coarse) {
    .settings-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
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
    /* Was `82vh`, which is the *tallest* the viewport ever gets — so on a phone with the URL
       bar out, or with the keyboard up over a token field, the panel was taller than the
       screen and the rows past the fold were unreachable. */
    /* `100%` of the overlay's content box, which is already the visible viewport less the
       top offset and both window insets. */
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
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
  .group {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--space-1);
  }
  .group .muted {
    font-weight: 400;
  }
  .filter {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--bg-inset, var(--bg));
    color: var(--fg);
    font: inherit;
  }
  /* **Not `.actions`** — that name was already taken by the dialog's footer button row, which
     is `display: flex`, so reusing it laid this list out horizontally and overflowed it off both
     edges of a phone. Found by screenshotting the emulator; no test would have seen it. */
  .binding-chip {
    margin-left: auto;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .binding-chip.unbound {
    border-style: dashed;
    color: var(--text-subtle);
    font-family: var(--font-sans);
  }
  .action-list {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
  }
  .action-list .sub {
    padding: var(--space-2) 0 2px;
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .action-list .action {
    /* Flex, not block: the label takes the room it needs and the binding chip is pushed to the
       far edge, which is where every menu on every platform puts an accelerator. */
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
    /* 2.75rem is the touch target both platform guidelines ask for; this list is the primary
       way into everything on a phone now. */
    min-height: 2.75rem;
    padding: var(--space-2);
    background: none;
    border: none;
    border-radius: var(--radius-2, 6px);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .action-list .action:hover {
    background: var(--surface-hover);
  }
  .binding {
    font-family: ui-monospace, monospace;
    font-size: var(--text-sm);
    padding: 2px 8px;
    min-width: 6rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--surface-elevated);
    color: var(--text);
    cursor: pointer;
  }
  .binding.capturing {
    border-color: var(--accent);
    color: var(--text-muted);
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

  /* The one editable vault setting: label and field on one line, so it reads as a sentence
     rather than a form. */
  .assets {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }
  .assets input {
    width: 6rem;
    padding: 4px 6px;
    font: inherit;
    font-size: var(--text-sm);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
  }
  .muted.small {
    font-size: var(--text-xs, 0.75rem);
    margin-top: 2px;
  }
  .assets-error {
    color: var(--danger, #b91c1c);
    font-size: var(--text-sm);
  }
  /* The pairing code and the certificate fingerprint are both read off this screen and compared
     against another one, character by character. Monospace and generous tracking are not
     decoration here — they are what makes `8`/`B` and `5`/`S` distinguishable at a glance. */
  .code {
    display: inline-block;
    margin-top: var(--space-2);
    font-family: var(--font-mono, monospace);
    letter-spacing: 0.12em;
    font-size: 1rem;
  }
  .fp {
    /* A fingerprint is 95 characters; it has to wrap somewhere, and mid-byte is unreadable. */
    word-break: break-all;
    letter-spacing: 0.02em;
    font-size: 0.8rem;
    line-height: 1.6;
  }
  .err {
    color: var(--danger, #b91c1c);
  }
</style>

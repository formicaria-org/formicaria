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
  import ManualLink from './ManualLink.svelte';
  import { onDestroy, onMount, tick } from 'svelte';
  import type { UpdateStatus } from './ipc';
  import { isPhone } from './platform';
  import * as nav from './nav.svelte';
  import { autoBackupOn, setAutoBackup } from './autoBackup';
  import {
    CATEGORIES,
    isCat,
    labelOf,
    shown as sectionShown,
    type Cat,
    type Place,
  } from './settingsMap';
  import Appearance from './Appearance.svelte';
  import CalendarsSection from './CalendarsSection.svelte';
  import TrackersSection from './TrackersSection.svelte';
  import RemindersSection from './RemindersSection.svelte';
  import MailSection from './MailSection.svelte';
  import BackupPanel from './BackupPanel.svelte';
  import {
    config as fetchConfig,
    setGitAssetsMax,
    checkMoveVault,
    moveVault,
    setSupervision,
    agentStatus,
    setAssistantName,
    agentModels,
    chooseAgentModel,
    removeAgentModel,
    updateRollback,
    reloadPage,
    updateStatus,
    setAgent,
    updateAgentModel,
    setTranscribe,
    setListening,
    alive,
    setShare,
    shareCode,
    revokeDevices,
    installOnPhone,
    phoneCanInstall,
    setUpdateCheck,
    updateApply,
    updateCancel,
    updateCheckNow,
    updateStart,
  } from './ipc';
  import { isRemote, shareSummary, type ShareStatus } from './remote';
  import type { Config } from './types';
  // **A vault's name is not what it is called.** Two devices that cloned one repository name
  // the same audience differently — this machine says `vault` where the phone says `notes` — so
  // every surface shows the repository behind the remote instead. This panel was displaying the
  // raw name, so the same vault read as two different vaults depending on which screen you opened
  // (reported 2026-09-08). See `vaults.svelte.ts`.
  import { labelFor, setVaults, vaultList, refreshVaults } from './vaults.svelte';
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
      setVaults(await setSupervision(vault, next));
    } catch (e) {
      consentError = e instanceof Error ? e.message : String(e);
    }
  }

  /** Write the vault's attachment limit and take the refreshed list back. The backend parses the
   *  size, so a typo is refused there and reported here rather than being half-applied. */
  async function setAssetMax(vault: string, raw: string) {
    assetError = null;
    try {
      setVaults(await setGitAssetsMax(vault, raw.trim()));
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
    onnewvault,
    onvaults,
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
    category,
    onkeyschanged,
  }: {
    onclose: () => void;
    /** Opens the new-vault form — create, join, restore or import. */
    onnewvault: () => void;
    /** Told when the set of vaults changed here (one removed, one taken back). */
    onvaults?: () => void;
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
    /** The category to open on, when the caller knows it; otherwise the one last used. */
    category?: import('./settingsMap').Cat;
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

  /// **What the screen keeps for itself** — the camera cutout, the status bar, the home bar.
  ///
  /// Here because **it is the only way to see it**. A phone renders its layout against these four
  /// numbers, and on a user's device the app's own logs are unreadable — `known-issues.md`:
  /// *"Rust's stderr is not routed to logcat on Android at all, and some vendors' builds suppress app logcat
  /// output besides."* When they reported (2026-09-08) that the top strip was being used, neither
  /// of us could tell whether the shell's inset bridge had delivered or the stylesheet's fallback
  /// was in force — the difference between a bug and a too-small guess, and no surface said which.
  ///
  /// **Measured, not read.** `getPropertyValue('--safe-top')` hands back the *specified* token —
  /// `max(env(safe-area-inset-top, 0px), 2.75rem)` — because a custom property is substituted
  /// rather than computed. Only laying it out resolves it, so this puts the four values on a hidden
  /// probe's padding and reads that back: whatever finally won, including the shell's inline
  /// override, is what appears.
  let insets = $state<{ top: number; right: number; bottom: number; left: number } | null>(null);
  function measureInsets() {
    if (typeof document === 'undefined') return;
    const probe = document.createElement('div');
    probe.style.cssText =
      'position:absolute;visibility:hidden;pointer-events:none;top:0;left:0;width:0;height:0;' +
      'padding-top:var(--safe-top);padding-right:var(--safe-right);' +
      'padding-bottom:var(--safe-bottom);padding-left:var(--safe-left);';
    document.body.appendChild(probe);
    const cs = getComputedStyle(probe);
    const px = (v: string) => Math.round(parseFloat(v) || 0);
    insets = {
      top: px(cs.paddingTop),
      right: px(cs.paddingRight),
      bottom: px(cs.paddingBottom),
      left: px(cs.paddingLeft),
    };
    probe.remove();
  }

  onMount(async () => {
    measureInsets();
    // The vault list, if this panel is the first thing to want it.
    void refreshVaults();
    // Land on the requested heading, so a "+" button arrives where it meant to rather than at
    // the top of a long panel.
    try {
      cfg = await fetchConfig();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    // Whether there is a version to go back to. **Local only** — this asks the server what is on
    // disk and reads a cached answer about releases; it never reaches the network, because opening
    // Settings must not be a reason to.
    try {
      const u = await updateStatus();
      upd = u;
      previousVersion = u.previous;
      canInstall = u.can_install;
      phoneBridge = phoneCanInstall();
      // A download already under way when the panel opened: keep watching it, the same courtesy the
      // assistant's first-enable download gets.
      if (inFlight(u.progress)) watchUpdate();
    } catch {
      // A build without the updater answers nothing. Hide the row rather than offer a control that
      // cannot work — the same rule the assistant's `agentOn = null` follows.
      previousVersion = null;
    }
    // The study-assistant toggle. `null` = unavailable here (e.g. a build without the agent), which
    // hides the row rather than showing a control that does nothing.
    try {
      const st = await agentStatus();
      myrmeName = st.device ?? '';
      agentOn = st.enabled;
      transcribeOn = st.transcribe;
      listeningBetter = st.listening_better ?? false;
      listeningBetterBytes = st.listening_better_bytes ?? null;
      agentInstalled = st.installed;
      provisioned = st.provisioned;
      provisionedBytes = st.provisioned_bytes;
      provisioning = st.provisioning;
      modelOffer = st.update ?? null;
      modelUnsupported = st.unsupported ?? false;
      if (st.installed) void loadPhoneModels();
      if (
        st.provisioning &&
        st.provisioning.stage !== 'ready' &&
        st.provisioning.stage !== 'restart' &&
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
  /// This device's name for its Myrme (2026-10-18): `@Myrme-<name>` asks this one, and what is
  /// written here is stamped with it. Applies when the assistant next starts.
  let myrmeName = $state('');
  let myrmeSaid = $state('');
  async function saveMyrmeName() {
    try {
      const word = await setAssistantName(myrmeName);
      myrmeName = word;
      myrmeSaid = `Saved. @Myrme-${word} asks this one; it applies when the assistant next starts.`;
    } catch (e) {
      myrmeSaid = e instanceof Error ? e.message : String(e);
    }
  }
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
  // Better listening (2026-10-18): a larger speech model, per device — slower, closer to what was
  // said. Shown only where the catalogue offers it, with what it downloads.
  let listeningBetter = $state(false);
  let listeningBetterBytes = $state<number | null>(null);
  async function toggleListening(next: boolean) {
    try {
      await setListening(next);
      listeningBetter = next;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
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
  /// What this copy could go back to, or null when there is nothing to go back to. Read from the
  /// server rather than remembered, so it stays true across a restart and a fresh clone.
  let autoBackup = $state(autoBackupOn());
  let previousVersion = $state<string | null>(null);
  /// What the earlier program is called in the *Go back* row. A copy built from source has no
  /// version, so the folder it is kept in is named `previous`; say what it is instead.
  const previousName = $derived(
    previousVersion === 'previous' ? 'the build made here' : previousVersion,
  );
  /// Whether this copy may install anything at all — a folder the system protects, or one whose
  /// launcher predates the way back, can say what it is but not change it. Separate from
  /// `previousVersion` for the reason the assistant's `agentOn`/`agentInstalled` pair is separate:
  /// a stored fact cannot fail, and conflating the two is how a panel offers a control that
  /// reports success and does nothing.
  let canInstall = $state(false);
  /// Armed by the first click, acted on by the second — the same two steps as removing the model,
  /// and for the same reason: this replaces the running program and cannot be undone from here.
  let goingBack = $state(false);
  /// The update picture as the server last described it. `null` hides every update row — a build with
  /// no updater, or one whose status never answered.
  let upd = $state<UpdateStatus | null>(null);
  let updTimer: ReturnType<typeof setTimeout> | undefined;
  /// Set once **this person** pressed *Check now*. A failed *background* check is silent by design —
  /// an error on every offline start teaches people to dismiss errors — but an answer to a question
  /// somebody just asked is owed to them.
  let askedNow = $state(false);
  /// *Check now* is under way. Held here rather than read from the status, because both shells answer
  /// the command only once the check has finished — the status's own `checking` is false again by the
  /// time the panel sees it — so the button gave no sign of having been pressed, and on the phone the
  /// owner could not tell whether a tap had worked (2026-09-11).
  let looking = $state(false);
  /// Two steps before restarting, like *Go back* and *Remove the model…*: it replaces the running
  /// program. Installing forward is recoverable — the way back is kept — which is why one arming is
  /// enough and there is no third.
  let restartArmed = $state(false);
  let restarting = $state(false);
  /// Whether the Android installer bridge is here. Read once: the shell registers it before the page
  /// loads, and it cannot appear later.
  let phoneBridge = $state(false);
  let phoneMessage = $state<string | null>(null);
  let updateError = $state<string | null>(null);
  let goBackError = $state<string | null>(null);

  /// Armed by the first click, acted on by the second — the two-step `BackupPanel` already uses for
  /// "forget this vault", and for the same reason: this frees gigabytes and cannot be undone.
  let removingModel = $state(false);
  let removedNote = $state<string | null>(null);
  let provisioning = $state<Awaited<ReturnType<typeof agentStatus>>['provisioning']>(null);
  // **A newer model is offered, never fetched unasked.** An app update can change which model the
  // assistant should use; the one already here keeps working until the person chooses to switch.
  let modelOffer = $state<NonNullable<Awaited<ReturnType<typeof agentStatus>>['update']> | null>(
    null,
  );
  let modelUnsupported = $state(false);
  /// The download under way is a newer model, not a first enable — so "Stop" must not turn the
  /// assistant off.
  let updatingModel = $state(false);
  const downloading = $derived(
    !!provisioning &&
      provisioning.stage !== 'ready' &&
      provisioning.stage !== 'restart' &&
      provisioning.stage !== 'failed',
  );
  let pollTimer: ReturnType<typeof setTimeout> | undefined;

  const picked = $derived(catalogue.find((m) => m.name === pickedModel) ?? null);
  /** What this choice will actually download, projector included when it is wanted. */
  const pickedBytes = $derived(
    isPhone()
      ? modelBytes(picked)
      : (picked?.bytes ?? 0) + (pickVision && picked?.vision ? (picked?.mmproj_bytes ?? 0) : 0),
  );

  type CatalogueModel = Awaited<ReturnType<typeof agentModels>>[number];
  /** What a model downloads on a phone: the weights and their image reader, which a phone always
   *  fetches (it reads photos with the same model). */
  function modelBytes(m: CatalogueModel | null | undefined): number {
    return (m?.bytes ?? 0) + (m?.vision ? (m?.mmproj_bytes ?? 0) : 0);
  }

  // **The phone's model choice** (Myrme 0.6.0): "Faster" or "More accurate, slower", in the
  // catalogue's words. Picking the other one downloads it beside the one in use, which keeps working
  // until the new one is ready — the same path as accepting a newer model.
  let phoneModels = $state<CatalogueModel[]>([]);
  let switchTo = $state<string | null>(null);
  const runningModel = $derived(phoneModels.find((m) => m.running)?.name ?? null);
  /** The one shown as picked: the person's pick, else the one running, else — when the phone runs
   *  a model that is neither choice, as after an update — the default, so the offer reads as it. */
  const selectedModel = $derived(
    switchTo ?? runningModel ?? phoneModels.find((m) => m.default)?.name ?? null,
  );
  const switchModel = $derived(phoneModels.find((m) => m.name === selectedModel) ?? null);
  /** On a phone the model row is the one place to choose; the separate "newer model" offer would
   *  ask the same question again with a button that does not say which it picks. It stays only for
   *  a phone whose model is no longer supported, which has no model row to get going again. */
  const phoneChooses = $derived(isPhone() && provisioned && phoneModels.length > 1);

  async function loadPhoneModels() {
    if (!isPhone()) return;
    try {
      phoneModels = await agentModels();
    } catch {
      phoneModels = []; // no chooser rather than a broken one
    }
  }

  async function switchPhoneModel() {
    if (!switchModel) return;
    error = null;
    try {
      await chooseAgentModel(switchModel.name);
      updatingModel = true;
      provisioning = {
        stage: 'model',
        done: 0,
        total: modelBytes(switchModel) || null,
        error: null,
      };
      switchTo = null;
      watchProvisioning();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

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
        modelOffer = st.update ?? null;
        modelUnsupported = st.unsupported ?? false;
        if (
          st.provisioning &&
          st.provisioning.stage !== 'ready' &&
          st.provisioning.stage !== 'restart' &&
          st.provisioning.stage !== 'failed'
        ) {
          watchProvisioning();
        } else {
          updatingModel = false;
          void loadPhoneModels(); // which one runs now
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

  /** Download the newer model that was offered. The one in use keeps working meanwhile. */
  async function acceptModel() {
    error = null;
    try {
      await updateAgentModel();
      updatingModel = true;
      provisioning = { stage: 'model', done: 0, total: modelOffer?.bytes ?? null, error: null };
      watchProvisioning();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  /** "Not now": the offer goes away until the model changes again. */
  async function declineModel() {
    error = null;
    try {
      await updateAgentModel({ dismiss: true });
      modelOffer = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
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

  onDestroy(() => clearTimeout(updTimer));

  function inFlight(p: UpdateStatus['progress']): boolean {
    return !!p && p.stage !== 'ready' && p.stage !== 'failed';
  }

  /** What each stage is doing, in words. Not a combined percentage: the steps are not comparable in
   *  size, and inventing one figure across them would be making something up. */
  const STAGE_WORDS: Record<string, string> = {
    manifest: 'checking what it contains',
    download: 'downloading',
    unpack: 'checking it runs on this computer',
  };

  /** Poll while getting it. The app has no streaming anywhere; this is the cadence the assistant's
   *  own download already uses, and a failed poll is not a failed download. */
  function watchUpdate() {
    clearTimeout(updTimer);
    updTimer = setTimeout(async () => {
      try {
        upd = await updateStatus();
      } catch {
        // Keep watching: one missed answer says nothing about the download.
      }
      if (inFlight(upd?.progress)) watchUpdate();
    }, 1500);
  }

  /** After *Restart* or *Go back*: wait for the other version to answer, then load the page from it.
   *
   *  The row said "this page reconnects by itself" and nothing made it true: the message stayed for
   *  ever, over an app that had restarted long before (a user's desktop, 2026-10-11), and the
   *  tab kept the old version's interface. A missed answer is the restart itself, so it keeps
   *  asking; it stops only when the program that answers is no longer the one that was running
   *  (`was`: its version, or null for a build that has none). */
  function watchRestart(was: string | null) {
    clearTimeout(updTimer);
    updTimer = setTimeout(async () => {
      try {
        if ((await updateStatus()).current !== was) return reloadPage();
      } catch {
        // Nobody answering yet: that is what a restart looks like.
      }
      watchRestart(was);
    }, 1500);
  }

  // ---- One category at a time (`settingsMap.ts`; `decisions.md`, 2026-10-11) ----
  //
  // The sections stay mounted and are shown or hidden: nothing is created or destroyed when the
  // category changes, so a download keeps reporting, a half-typed field keeps its text, and a
  // link to a section finds it whichever category was open. Each section says where it lives
  // (`data-cat`, on itself or on what wraps it); this applies the answer.
  const CAT_KEY = 'fm-settings-category';
  /** The ＋ asks for the actions; they are a window of their own, not a part of Settings. */
  const actionsMode = $derived(section === 'commands');
  let panelEl = $state<HTMLElement | null>(null);
  // svelte-ignore state_referenced_locally
  let cat = $state<Cat>(category ?? lastCategory());
  let query = $state('');
  /** On a narrow screen: is a category open (true), or the list of them (false)? */
  // svelte-ignore state_referenced_locally
  let opened = $state(!!category);
  let present = $state<Cat[]>([]);
  let found = $state(0);
  const atList = $derived(!actionsMode && !opened && !query.trim());
  function lastCategory(): Cat {
    try {
      const was = localStorage.getItem(CAT_KEY);
      return isCat(was) ? was : 'look';
    } catch {
      return 'look';
    }
  }
  function choose(c: Cat) {
    cat = c;
    query = '';
    opened = true;
    steppedIn = true;
    try {
      localStorage.setItem(CAT_KEY, c);
    } catch {
      /* it opens on Look next time */
    }
    if (panelEl) panelEl.scrollTop = 0;
  }
  function backToList() {
    opened = false;
    query = '';
    steppedIn = false;
  }

  /// **On a narrow screen a category is a second screen, so Back returns to the list** (the owner,
  /// 2026-10-20: About & updates, then Back, closed Settings). Only a category the person stepped
  /// into: one a link opened Settings on was never under a list they saw. On a wide screen the list
  /// stays beside the category, and Back there closes Settings as before.
  const NARROW = '(max-width: 46rem)'; // the breakpoint of the styles below
  const narrowQuery = typeof matchMedia === 'function' ? matchMedia(NARROW) : null;
  let narrow = $state(narrowQuery?.matches ?? false);
  $effect(() => {
    const on = () => (narrow = narrowQuery?.matches ?? false);
    narrowQuery?.addEventListener?.('change', on);
    return () => narrowQuery?.removeEventListener?.('change', on);
  });
  let steppedIn = $state(false);
  nav.level(
    () => 'settings-category',
    () => steppedIn && opened && narrow && !actionsMode,
    backToList,
  );
  /** What a link asked for (`section`), honoured once its section exists. */
  let landed = false;
  let backupMounted = $state(false);

  // ── moving a vault's folder (`decisions.md#vault`, 2026-10-15) ──────────────────────────────
  /** Which vault's move form is open, if any — one at a time. */
  let moving = $state<string | null>(null);
  let moveTo = $state('');
  let moveCheck = $state<import('./ipc').MoveCheck | null>(null);
  let moveBusy = $state(false);
  let moveError = $state<string | null>(null);
  let moved = $state<import('./ipc').Moved | null>(null);
  /** Not where vaults live in formicaria's own storage (a phone), and not from a paired device:
   *  there is no folder of the person's to choose. The server refuses both anyway. */
  const canMove = $derived(!isRemote() && !!cfg && cfg.vault_root === null);
  let moveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const [v, to] = [moving, moveTo];
    clearTimeout(moveTimer);
    moveCheck = null;
    if (!v || !to.trim()) return;
    moveTimer = setTimeout(async () => {
      moveCheck = await checkMoveVault(v, to).catch((e) => ({ ok: false, problem: reason(e) }));
    }, 300);
    return () => clearTimeout(moveTimer);
  });
  function startMove(name: string) {
    moving = name;
    moveTo = '';
    moveError = null;
    moved = null;
  }
  async function doMove() {
    if (!moving || !moveCheck?.ok || moveBusy) return;
    moveBusy = true;
    moveError = null;
    try {
      moved = await moveVault(moving, moveTo);
      moving = null;
      await refreshVaults();
      onvaults?.();
    } catch (e) {
      moveError = reason(e);
    } finally {
      moveBusy = false;
    }
  }
  function applyView() {
    const el = panelEl;
    if (!el) return;
    if (section && !actionsMode && !landed) {
      const target = document.getElementById(section);
      const where = target?.closest<HTMLElement>('[data-cat]')?.dataset.cat;
      if (target && isCat(where)) {
        landed = true;
        cat = where;
        opened = true;
        // After the category is drawn: scrolling to an element that is still hidden does nothing,
        // which is how a link landed on the category but not on Vaults.
        void tick().then(() =>
          requestAnimationFrame(() => target.scrollIntoView({ block: 'start' })),
        );
      }
    }
    const secs = [...el.querySelectorAll<HTMLElement>('section, p.group[data-cat]')].flatMap(
      (sec) => {
        const place = sec.closest<HTMLElement>('[data-cat]')?.dataset.cat as Place | undefined;
        return place ? [{ sec, place, heading: sec.tagName === 'P' }] : [];
      },
    );
    const have = new Set<Cat>();
    for (const { place, heading } of secs) if (place !== 'actions' && !heading) have.add(place);
    const now = CATEGORIES.map((c) => c.id).filter((c) => have.has(c));
    if (now.join() !== present.join()) present = now;
    // The chosen category may not be here (yet: the assistant's section arrives with its answer;
    // or at all, on a copy without it). Show the first that is, and keep the choice.
    const showing = have.has(cat) ? cat : (now[0] ?? cat);
    // The backup half of Vaults asks git about every vault's remote, which is a network call per
    // vault — so it is mounted the first time its category is shown or a search could reach it,
    // never merely because Settings opened (`decisions.md#ui`, 2026-10-15).
    if (!backupMounted && !actionsMode && (showing === 'vaults' || query.trim())) {
      backupMounted = true;
    }
    let hits = 0;
    for (const { sec, place, heading } of secs) {
      const show = sectionShown({
        actions: actionsMode,
        cat: showing,
        query,
        place,
        // What a section is also called, by someone who does not know our word for it.
        text: `${sec.textContent ?? ''} ${sec.dataset.words ?? ''}`,
        heading,
      });
      sec.hidden = !show;
      // While searching, each result says which category it is in.
      if (show && query.trim() && place !== 'actions') {
        sec.dataset.found = labelOf(place);
        hits++;
      } else {
        delete sec.dataset.found;
      }
    }
    found = hits;
  }
  $effect(() => {
    const el = panelEl;
    if (!el) return;
    void cat;
    void query;
    void actionsMode;
    applyView();
    // Sections arrive as their answers do (the assistant, sharing, the vault list).
    const seen = new MutationObserver(applyView);
    seen.observe(el, { childList: true, subtree: true });
    return () => seen.disconnect();
  });
  /** What each category is set to right now, said beside its name. */
  const states = $derived.by<Partial<Record<Cat, string>>>(() => ({
    look: `${theme.charAt(0).toUpperCase()}${theme.slice(1)} · ${
      layout === 'tiled' ? 'Tiled' : 'One view'
    }`,
    vaults: `${vaults.length} vault${vaults.length === 1 ? '' : 's'} · backs up by itself ${
      autoBackup ? 'on' : 'off'
    }`,
    assistant: agentOn === null ? undefined : agentOn ? 'On' : 'Off',
    about: cfg ? (upd?.available ? `${cfg.version} · a newer one is out` : cfg.version) : undefined,
  }));

  function reason(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  async function checkNow() {
    askedNow = true;
    updateError = null;
    looking = true;
    try {
      upd = await updateCheckNow();
    } catch (e) {
      updateError = reason(e);
    } finally {
      looking = false;
    }
  }

  /** When the last look was, to the minute. A date alone read the same before and after a second
   *  look that day, so a press that worked could not be told from one that did nothing. */
  function lookedWhen(secs: number): string {
    const then = new Date(secs * 1000);
    const time = then.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    return then.toDateString() === new Date().toDateString()
      ? `today at ${time}`
      : `${then.toLocaleDateString()} at ${time}`;
  }

  async function toggleUpdateCheck(on: boolean) {
    updateError = null;
    try {
      upd = await setUpdateCheck(on);
    } catch (e) {
      updateError = reason(e);
    }
  }

  async function getUpdate() {
    updateError = null;
    phoneMessage = null;
    try {
      upd = await updateStart();
      watchUpdate();
    } catch (e) {
      updateError = reason(e);
    }
  }

  async function stopUpdate() {
    clearTimeout(updTimer);
    try {
      upd = await updateCancel();
    } catch (e) {
      updateError = reason(e);
    }
  }

  /** The server saves what is on disk, answers, and only then stops — so this press lands before the
   *  connection goes, and `App`'s reconnecting surface takes over the gap. */
  async function restartIntoUpdate() {
    updateError = null;
    try {
      const was = upd?.current ?? null;
      await updateApply();
      restarting = true;
      watchRestart(was);
    } catch (e) {
      updateError = reason(e);
    } finally {
      restartArmed = false;
    }
  }

  /** Android: ask the shell to open the system installer on the download it already checked. */
  function installOnThisPhone() {
    updateError = null;
    phoneMessage = null;
    const r = installOnPhone();
    if (r === 'ok') {
      phoneMessage =
        'The installer has opened. Confirm there, and formicaria restarts as the new version with your notes as they are.';
    } else if (r === 'permission') {
      phoneMessage =
        'Android needs your permission first: allow installs from formicaria on the screen that just opened, then come back and press Install again.';
    } else if (r) {
      updateError = r.replace(/^error:\s*/, '');
    }
  }

  /** Put the previous version back. The server answers before it stops, so the reply lands and then
   *  the connection goes; `App` already shows its reconnecting surface when that happens, which is
   *  why there is nothing to do on success but say what is happening. */
  async function goBack() {
    goBackError = null;
    try {
      const was = upd?.current ?? null;
      await updateRollback();
      goingBack = false;
      previousVersion = null;
      watchRestart(was);
    } catch (e) {
      goingBack = false;
      goBackError = e instanceof Error ? e.message : String(e);
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
      if (updatingModel) {
        // Stopping the download of a newer model leaves the assistant exactly as it was.
        await updateAgentModel({ cancel: true });
        updatingModel = false;
        provisioning = null;
        return;
      }
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

  /// **The vault list, read from the one store rather than out of `config`.** This panel used to
  /// keep its own copy inside `cfg` and splice writers' return values into it — which left
  /// `cfg.restic` stale, since only the `vaults` half was ever replaced. `mock.contract.test.ts`
  /// exists because `config` and `backup_status` had already disagreed about exactly that field.
  const vaults = $derived(vaultList() ?? []);
  const resticFor = (name: string) => vaults.find((v) => v.name === name)?.restic_repo ?? null;

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

{#snippet vaultRow(name: string)}
  {@const v = vaults.find((x) => x.name === name)}
  {#if v}
    <div class="line">
      <strong>{labelFor(v.name)}</strong>
      <!-- **The name too, wherever it differs from the label** — and only here, because this
                   screen is the one that answers "what am I operating with?". Everywhere else the
                   label is right: it is what two devices agree on. But the *name* is the identity —
                   the argument every command takes, and on a phone the address a vault is created
                   at (`resolve_path`) — so routing every surface through `labelFor` (2026-09-08)
                   left the one string needed to add a vault back displayed nowhere at all. The
                   owner hit exactly that within the hour: Settings said `formicarium-vault`, the
                   new-vault form wanted `notes`, and nothing on screen connected them. -->
      {#if labelFor(v.name) !== v.name}
        <span class="muted small">name: {v.name}</span>
      {/if}
      {#if v.default}<span class="tag">default</span>{/if}
    </div>
    <!-- Been on the wire since `list_vaults` existed and rendered nowhere until now.
                 "Which folder is this actually?" had no answer inside the app. -->
    <p class="path"><code>{v.path}</code></p>
    {#if moved?.vault === v.name}
      <p class="muted small" role="status">
        Moved from <code>{moved.from}</code> — {moved.files} files, {humanSize(
          moved.bytes,
        )}{moved.copied
          ? ', copied and checked file by file'
          : ''}.{#if moved.note}{' '}{moved.note}{/if}
      </p>
    {/if}
    {#if canMove}
      {#if moving === v.name}
        <div class="move">
          <label class="line">
            <span>Move it to</span>
            <input
              bind:value={moveTo}
              placeholder={`~/notes/${v.name}`}
              autocomplete="off"
              spellcheck="false"
              autocapitalize="off"
              aria-label={`new folder for ${labelFor(v.name)}`}
              disabled={moveBusy}
            />
          </label>
          {#if moveCheck?.ok}
            <p class="muted small">
              {moveCheck.files} files, {humanSize(moveCheck.bytes ?? 0)}, will move to
              <code>{moveCheck.to}</code>. Its name, its place in the list and its settings stay. On
              the same disk this is instant; on another, every file is copied and checked before the
              old folder is removed — if anything fails, it stays where it is.
            </p>
          {:else if moveCheck?.problem}
            <p class="warn">{moveCheck.problem}</p>
          {/if}
          {#if moveError}<p class="assets-error">{moveError}</p>{/if}
          <div class="line">
            <button onclick={() => void doMove()} disabled={!moveCheck?.ok || moveBusy}>
              {moveBusy ? 'Moving…' : 'Move'}
            </button>
            <button class="link" onclick={() => (moving = null)} disabled={moveBusy}>Cancel</button>
          </div>
        </div>
      {:else}
        <button class="link" onclick={() => startMove(v.name)}>Move to another folder…</button>
      {/if}
    {/if}
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
        aria-label={`largest attachment to send for ${labelFor(v.name)}`}
      />
    </label>
    <p class="muted small">
      {#if v.git_assets_max}
        Files up to {humanSize(Math.min(v.git_assets_max, GIT_ASSETS_CEILING))} are pushed with your notes.
        Anything larger stays on this device — git history is permanent, so a large file committed once
        is in every clone forever.
      {:else}
        Empty means <strong>notes only</strong> — the default. Attachments stay in the vault and
        travel only via restic. The most you can send this way is
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
        This vault asks for {humanSize(v.git_assets_max)}, which is more than will ever be sent:
        attachments over {humanSize(GIT_ASSETS_CEILING)} are left out, because most hosts refuse a file
        that size and the push would fail after the commit was made. Edit
        <code>vault.json</code> to agree, or leave the heavy ones to the backup snapshot.
      </p>
    {:else if v.git_assets_max && v.git_assets_max > GIT_ASSETS_WARN}
      <p class="warn">
        That is large for git. Files this size are kept forever, downloaded again by every clone,
        and cannot be taken back without rewriting history other people have already pulled — and
        many hosts warn above {humanSize(GIT_ASSETS_WARN)}. The backup snapshot carries attachments
        of any size without any of that.
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
          Records made from now on are marked as shareable — sharing itself is still a separate,
          deliberate step, and it cannot be taken back once taken.
        {:else}
          Sharing is off, so nothing here can be published.
        {/if}
      {:else}
        Off — nothing is recorded about what you change.
      {/if}
    </p>
  {/if}
{/snippet}

<svelte:window onkeydown={(e) => (capturing ? capture(e) : onkeydown(e))} />

<div class="settings-overlay">
  <button class="settings-backdrop" aria-label="close settings" onclick={onclose}></button>
  <div
    class="panel"
    class:actions-only={actionsMode}
    class:at-list={atList}
    class:searching={!!query.trim()}
    role="dialog"
    aria-label={actionsMode ? 'actions' : 'settings'}
    bind:this={panelEl}
  >
    <div class="panel-head">
      {#if !actionsMode && !atList}
        <button class="back" aria-label="all settings" onclick={backToList}>‹</button>
      {/if}
      {#if actionsMode}
        <h2>Do something</h2>
      {:else}
        <!-- On a narrow screen an open category is a screen of its own, and says its name. The
             window is still called Settings to anything that asks for it by name. -->
        <h2>
          <span class="t-all">Settings</span><span class="t-cat" aria-hidden="true"
            >{labelOf(cat)}</span
          >
        </h2>
      {/if}
      {#if !actionsMode}
        <!-- **Search every category at once.** Typing here shows whatever matches, from anywhere
             in Settings, each under the name of the category it lives in. -->
        <input
          class="find"
          type="search"
          placeholder="Search settings"
          aria-label="search settings"
          autocomplete="off"
          spellcheck="false"
          bind:value={query}
        />
      {/if}
      <button class="x" aria-label="close" title="Close" onclick={onclose}>✕</button>
    </div>
    <!-- **The categories.** One is on screen at a time; each says its current state beside its
         name, so the list is also the overview. One this copy has nothing in is not listed. -->
    {#if !actionsMode}
      <nav class="jump" aria-label="settings categories">
        {#each CATEGORIES.filter((c) => present.includes(c.id)) as c (c.id)}
          <button class:on={c.id === cat && !query.trim()} onclick={() => choose(c.id)}>
            <span class="cat-name">{c.label}</span>
            {#if states[c.id]}<span class="cat-state">{states[c.id]}</span>{/if}
          </button>
        {/each}
      </nav>
    {/if}
    {#if !actionsMode && query.trim() && found === 0}
      <p class="muted nothing">Nothing in Settings matches “{query.trim()}”.</p>
    {/if}

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
      <p class="group" id="commands" data-cat="actions">Do something</p>

      <section data-cat="actions">
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

      <p class="group" data-cat="look">
        Preferences <span class="muted">— this browser, no vault touched</span>
      </p>

      <section data-cat="look" data-words="dark light mode colour color">
        <h3>Theme</h3>
        <ManualLink page="user/appearance.html" section="themes-and-darklight" />
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
      <div class="cat" data-cat="look">
        <Appearance selected={userTheme} onselect={onusertheme} />
      </div>

      <section data-cat="look">
        <h3>Layout</h3>
        <ManualLink page="user/workspace.html" section="one-view-or-several" />
        <p class="muted lede">One view at a time, or several side by side.</p>
        <details class="more">
          <summary>More</summary>
          <p class="muted">
            How views are arranged. <strong>One view</strong> is the default everywhere, including this
            machine — a narrow layout only a phone could run would be a second frontend wearing a setting,
            and nothing would exercise it during ordinary desktop work.
          </p>
        </details>
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
        <section data-cat="assistant" data-words="ai model llm transcription agent">
          <h3>Study assistant</h3>
          <ManualLink page="user/assistant.html" section="turn-it-on" />
          <p class="muted lede">
            A small model on this device that answers in discussions and proposes edits you review.
          </p>
          <details class="more">
            <summary>More</summary>
            <p class="muted">
              <strong>Myrme</strong>, a small local model that reads your notes and answers in
              discussions — write <code>@Myrme</code>, its name on every device, and it replies; it
              can propose edits you review, and never touches your notes on its own. It runs
              entirely on this device, starts and stops
              <strong>with formicaria</strong>, and is <strong>off by default</strong>. A change
              takes effect at the next launch.
            </p>
          </details>
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
              {#if agentOn}
                <!-- **One Myrme per device, each with a name** (2026-10-18). `@Myrme` is answered by
                     the device a message was written on; this name lets a person ask one device's
                     copy from another, and tells them apart in what they sign. -->
                <li>
                  <label class="choice">
                    <span class="k">This device's Myrme</span>
                    <input
                      class="field"
                      type="text"
                      aria-label="this device's name for Myrme"
                      placeholder="laptop, phone, office desktop…"
                      bind:value={myrmeName}
                      onkeydown={(e) => e.key === 'Enter' && void saveMyrmeName()}
                      autocomplete="off"
                      spellcheck="false"
                    />
                    <button type="button" onclick={() => void saveMyrmeName()}>Save</button>
                  </label>
                  <span class="muted">
                    {myrmeSaid ||
                      `@Myrme-${myrmeName || 'laptop'} asks this device's copy from another; plain @Myrme asks the one where you write.`}
                  </span>
                </li>
              {/if}

              <!-- **Ask, with the numbers, before spending anything.** The size and the licence are
                 read from the catalogue rather than written here, so the figures cannot drift from
                 what is actually fetched. -->
              {#if choosing}
                <li class="vault">
                  <p class="why">
                    The assistant needs a model on this {isPhone() ? 'phone' : 'computer'}. It is
                    downloaded once, kept on this {isPhone() ? 'phone' : 'machine'}, and used
                    offline afterwards. Pick one:
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
                      <span class="k">{m.label ?? m.name}</span>
                      <span class="muted">
                        {m.label ? `${m.name} · ` : ''}{m.bytes
                          ? humanSize(isPhone() ? modelBytes(m) : m.bytes)
                          : 'size unknown'}{m.license ? ` · ${m.license}` : ''}{m.vision &&
                        !isPhone()
                          ? ' · can read images'
                          : ''}
                      </span>
                    </label>
                  {/each}
                  {#if picked?.vision && !isPhone()}
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
              {#if phoneChooses && !downloading}
                <li>
                  <span class="k">model</span>
                  {#each phoneModels as m (m.name)}
                    <label class="choice">
                      <input
                        type="radio"
                        name="fm-phone-model"
                        value={m.name}
                        checked={selectedModel === m.name}
                        onchange={() => (switchTo = m.name)}
                      />
                      <span class="k">{m.label ?? m.name}</span>
                      <span class="muted">{m.running ? 'in use' : humanSize(modelBytes(m))}</span>
                    </label>
                  {/each}
                  {#if switchModel && switchModel.name !== runningModel}
                    <button class="primary" onclick={switchPhoneModel}>
                      Download {humanSize(modelBytes(switchModel))} and switch
                    </button>
                    {#if switchTo}
                      <button onclick={() => (switchTo = null)}>Cancel</button>
                    {/if}
                  {/if}
                </li>
              {/if}
              {#if modelOffer && !downloading && provisioning?.stage !== 'restart' && (!phoneChooses || modelUnsupported)}
                <li>
                  <span class="k">newer model</span>
                  <span class="muted">
                    {modelUnsupported
                      ? 'The model on this device no longer works with this version of formicaria.'
                      : 'This version of formicaria comes with a newer model for the assistant.'}
                    <strong>{modelOffer.name}</strong>{modelOffer.bytes
                      ? `, a ${humanSize(modelOffer.bytes)} download`
                      : ''}{modelOffer.license ? ` (${modelOffer.license})` : ''}.
                    {modelOffer.replaces
                      ? `You are using ${modelOffer.replaces}, which keeps working until you switch.`
                      : ''}
                  </span>
                  <button class="primary" onclick={acceptModel}>Download</button>
                  {#if !modelUnsupported}
                    <button onclick={declineModel}>Not now</button>
                  {/if}
                </li>
              {/if}
              {#if provisioning?.stage === 'restart'}
                <li>
                  <span class="k">new model ready</span>
                  <span class="muted">
                    It is downloaded and takes over the next time you open formicaria. The old one
                    is removed then.
                  </span>
                </li>
              {/if}
              {#if provisioning && provisioning.stage !== 'ready' && provisioning.stage !== 'restart'}
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
                {#if transcribeOn && listeningBetterBytes}
                  <li>
                    <label class="choice">
                      <input
                        type="checkbox"
                        checked={listeningBetter}
                        onchange={(e) => toggleListening(e.currentTarget.checked)}
                      />
                      <span class="k">Better listening {listeningBetter ? 'on' : 'off'}</span>
                      <span class="muted">
                        {listeningBetter
                          ? 'A larger speech model writes down more exactly what was said, in any language, and takes longer. Applies at the next assistant start.'
                          : `Writes down more exactly what was said — good for dictated recipes and long recordings. Downloads about ${Math.round(listeningBetterBytes / 1_000_000)} MB and takes longer; applies at the next assistant start.`}
                      </span>
                    </label>
                  </li>
                {/if}
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
        <section data-cat="connections" data-words="tablet phone pair pairing network">
          <h3>Share with another device</h3>
          <ManualLink page="user/sharing.html" section="turning-it-on" />
          <p class="muted lede">Open your notes on a tablet or phone on the same network.</p>
          <details class="more">
            <summary>More</summary>
            <p class="muted">
              Open your notes on a tablet or phone on the same network — useful for touch and for
              taking photos straight into a note. The notes stay <strong>on this computer</strong>;
              the other device reads and writes them over your own network, and nothing is sent
              anywhere else. <strong>Off by default</strong>, and a change takes effect at the next
              launch.
            </p>
          </details>
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
                  {#each vaults as v (v.name)}
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
                        <span class="k">{labelFor(v.name)}</span>
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

      <!-- Your own calendars. The computer holds the addresses and does the reading, so a paired
           tablet (refused every route) and the phone (no reader yet) are not offered it. -->
      {#if !isRemote() && !isPhone()}
        <div class="cat" data-cat="connections">
          <CalendarsSection />
          <MailSection />
        </div>
      {/if}

      <section data-cat="look" data-words="grid width panes">
        <h3>Columns</h3>
        <ManualLink page="user/workspace.html" section="columns" />
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

      <section data-cat="keyboard" data-words="shortcut shortcuts hotkey keys">
        <h3>Keyboard</h3>
        <ManualLink page="user/workspace.html" section="keyboard-shortcuts" />
        <p class="muted">
          Click a shortcut, then press the keys you want. <strong>Esc</strong> cancels. The
          <em>Next / Previous view</em> pair works even while you are typing in a note — which is
          the whole reason they exist, since otherwise a note has to be closed before anything else
          can be reached. The <em>in a note</em> keys format the words you are writing.
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
              {:else if keys.IN_NOTE.has(cmd) && keymap[cmd].key && !keys.chord(keymap[cmd])}
                <!-- It would type its letter instead: a note key fires while you write. -->
                <span class="warn">needs Ctrl or Alt</span>
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

      <p class="group" data-cat="about">
        This installation <span class="muted">— read-only; the backend owns these</span>
      </p>

      <section data-cat="about">
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

      <!-- **Vaults & backup, a category of its own** (`decisions.md#ui`, 2026-10-15): everything about
           a vault is set here and nowhere else. -->
      <section
        data-cat="vaults"
        data-words="backup back up sync send attachments vault vaults automatic new vault create import logseq obsidian join restore remote address token password restic snapshot remove forget"
      >
        <div class="vaults-head" id="vaults">
          <h3>Vaults <span class="count">{vaults.length}</span></h3>
          <ManualLink page="user/backup.html" section="setting-up-once" />
          {#if !isRemote()}
            <button class="new-vault" onclick={onnewvault}>New vault…</button>
          {/if}
        </div>
        <!-- **Backing up by itself.** On unless turned off, per device, and it says beside the
             switch what it does and when: an automatic thing a person cannot see described is one
             they did not agree to. Not offered to a paired device, which may not back up at all. -->
        {#if !isRemote()}
          <ul class="caps updates">
            <li>
              <label class="choice">
                <input
                  type="checkbox"
                  aria-label="back up by itself"
                  checked={autoBackup}
                  onchange={(e) => {
                    autoBackup = e.currentTarget.checked;
                    setAutoBackup(autoBackup);
                  }}
                />
                <span class="k">{autoBackup ? 'On' : 'Off'}</span>
                <span class="muted">
                  Backs up by itself: a little after you stop writing, and when you leave the app,
                  formicaria gets the changes from your other devices and sends yours. Never while
                  you are typing, and never a vault that has not been backed up once by you.
                </span>
              </label>
            </li>
          </ul>
        {/if}
        <!-- **One block per vault, with everything about it** (`decisions.md#ui`, 2026-10-15): these
             rows, then — once this category has been shown — where it is sent, its snapshot, and
             removing it, all drawn by the backup half below. A paired device has no backup half. -->
        {#if backupMounted && !isRemote()}
          <BackupPanel vaultRows={vaultRow} {onvaults} />
        {:else}
          {#each vaults as v (v.name)}
            <div class="vault">{@render vaultRow(v.name)}</div>
          {/each}
        {/if}
        {#if assetError}
          <p class="assets-error">{assetError}</p>
        {/if}
        {#if consentError}
          <p class="assets-error">{consentError}</p>
        {/if}
      </section>

      <!-- Food and training: a switch for this device, and the food list. Not offered to a paired
           device — the list is the host's disk, and what it shows is its own choice. -->
      {#if !isRemote()}
        <div
          class="cat"
          data-cat="notes"
          data-words="food meals calories nutrition diet training workout exercise tracker"
        >
          <TrackersSection />
        </div>
      {/if}

      <!-- Reminders: a switch for this device. -->
      <div class="cat" data-cat="notes" data-words="reminders notifications alerts deadline event">
        <RemindersSection />
      </div>

      <section data-cat="about" data-words="update updates upgrade version release about">
        <h3>This machine</h3>
        <ManualLink page="user/updating.html" />
        <ul class="caps facts">
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
          <!-- **The strip the app must not draw in.** Reported 2026-09-08 from a phone: *"you are
               continuing to use the top part of the screen which is untouchable and passes through
               the frontal camera… we cannot use top pixels."* Stated as a number because the two
               ways it can go wrong look identical on screen — the shell's bridge not delivering,
               and a fallback smaller than the device's cutout — and this is the only surface that
               can tell them apart on a device whose logs are suppressed. -->
          <li>
            <span class="k">screen edges kept clear</span>
            {#if insets && (insets.top || insets.bottom || insets.left || insets.right)}
              <code>top {insets.top} · bottom {insets.bottom}</code>
              {#if insets.left || insets.right}
                <code>sides {insets.left} / {insets.right}</code>
              {/if}
              — nothing is drawn under the camera or the home bar.
            {:else}
              <span class="none">none</span> — this screen reports no camera cutout and no system bars,
              so the app uses every pixel. Right on a desktop; on a phone it means the shell did not say,
              and something may sit under the lens.
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
              <span class="none">live only on this phone</span> until they reach a backup — backing up
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

        <!-- **Newer versions.** Checking is on by default and says, beside the switch, exactly
             what it sends — a network call a person cannot see described is not one they agreed to.
             Getting and installing are never automatic; each is a press. On a phone the last step
             belongs to Android's installer, so that row offers *Install* rather than a restart. -->
        {#if upd && upd.can_check}
          <ul class="caps updates">
            <li>
              <label class="choice">
                <input
                  type="checkbox"
                  aria-label="look for newer versions"
                  checked={upd.check}
                  onchange={(e) => toggleUpdateCheck(e.currentTarget.checked)}
                />
                <span class="k">{upd.check ? 'On' : 'Off'}</span>
                <span class="muted">
                  Once a day formicaria asks GitHub which version is newest. Nothing about you, this
                  computer or your notes is sent.
                </span>
              </label>
            </li>
            <li>
              {#if upd.available && upd.progress && inFlight(upd.progress)}
                <span class="k">getting {upd.available}</span>
                <span class="muted">
                  {STAGE_WORDS[upd.progress.stage] ?? upd.progress.stage}…
                  {#if upd.progress.stage === 'download'}
                    {upd.progress.total
                      ? `${humanSize(upd.progress.done)} of ${humanSize(upd.progress.total)}`
                      : humanSize(upd.progress.done)}
                  {/if}
                </span>
                <button onclick={stopUpdate}>Stop</button>
              {:else if upd.available && upd.progress?.stage === 'ready' && phoneBridge}
                <span class="k">{upd.available} is ready</span>
                <span class="muted">Downloaded and checked. Installing it keeps your notes.</span>
                <button class="primary" onclick={installOnThisPhone}>Install {upd.available}</button
                >
              {:else if upd.available && upd.progress?.stage === 'ready' && restarting}
                <span class="k">restarting</span>
                <span class="muted">
                  formicaria is restarting as {upd.available}. This page reconnects by itself.
                </span>
              {:else if upd.available && upd.progress?.stage === 'ready' && restartArmed}
                <span class="k">restart into {upd.available}?</span>
                <span class="muted">
                  What you have typed is saved first, and your notes are not touched. If
                  {upd.available} does not suit you, you can go back to {cfg.version} from here.
                </span>
                <button class="primary" onclick={restartIntoUpdate}>Yes, restart</button>
                <button onclick={() => (restartArmed = false)}>Cancel</button>
              {:else if upd.available && upd.progress?.stage === 'ready'}
                <span class="k">{upd.available} is ready</span>
                <span class="muted">Downloaded and checked. Restart to finish.</span>
                <button onclick={() => (restartArmed = true)}>Restart into {upd.available}…</button>
              {:else if upd.available && upd.progress?.stage === 'failed'}
                <span class="k">could not get {upd.available}</span>
                <span class="bad">{upd.progress.error}</span>
                <button onclick={getUpdate}>Try again</button>
              {:else if upd.available && upd.can_install}
                <span class="k">{upd.available} is available</span>
                <span class="muted">You have {cfg.version}.</span>
                <button onclick={getUpdate}>Get {upd.available}</button>
              {:else if upd.available}
                <span class="k">{upd.available} is available</span>
                <span class="muted">{upd.why}</span>
                {#if !isPhone() && upd.page}
                  <a class="link" href={upd.page} target="_blank" rel="noopener noreferrer"
                    >Open the download page</a
                  >
                {/if}
              {:else}
                <!-- The label is the answer and the sentence says when it was found, so neither
                     repeats the other: this read "newest version · You have the newest version". -->
                <span class="k">{upd.last_check ? 'up to date' : 'not looked yet'}</span>
                {#if upd.last_check}
                  <span class="muted">Last looked {lookedWhen(upd.last_check)}.</span>
                {/if}
                <button onclick={checkNow} disabled={looking || upd.checking}>
                  {looking || upd.checking ? 'Looking…' : 'Check now'}
                </button>
              {/if}
            </li>
            <!-- **An offered version must never be the end of the road.** *Check now* lived only in
                 the "up to date" state, so once a version was on offer there was no way to ask
                 again — and a copy that had remembered an old release kept offering it for ever
                 (a user's phone, 2026-10-11: v0.6.3 offered with v0.6.5 out, and nothing to
                 press but *Get v0.6.3*). It says when the offer was found, which is what makes a
                 stale one recognisable. Not while a download is in flight or sitting ready: looking
                 again then could put a new version's name on an old version's file. -->
            {#if upd.available && (!upd.progress || upd.progress.stage === 'failed')}
              <li>
                <span class="k">found</span>
                <span class="muted">
                  {upd.last_check ? `${lookedWhen(upd.last_check)}.` : 'earlier.'} A newer one may be
                  out since.
                </span>
                <button onclick={checkNow} disabled={looking || upd.checking}>
                  {looking || upd.checking ? 'Looking…' : 'Check now'}
                </button>
              </li>
            {/if}
            {#if updateError}
              <li><span class="bad">{updateError}</span></li>
            {/if}
            {#if askedNow && upd.error && !upd.checking}
              <li><span class="bad">Could not look just now: {upd.error}</span></li>
            {/if}
            {#if phoneMessage}
              <li><span class="muted">{phoneMessage}</span></li>
            {/if}
          </ul>
        {:else if upd}
          <!-- **A copy that cannot look says so.** This branch showed nothing, so a person saw no
               update rows and no reason — "why is there no Check now?" with nowhere to read the
               answer (the owner's Linux desktop, 2026-10-11). The reason is the app's own, and the
               download page is the way forward it can still offer. -->
          <ul class="caps updates">
            <li>
              <span class="k">newer versions</span>
              <span class="muted">{upd.why || 'This copy cannot look for a newer version.'}</span>
              {#if !isPhone() && upd.page}
                <a class="link" href={upd.page} target="_blank" rel="noopener noreferrer"
                  >Open the download page</a
                >
              {/if}
            </li>
          </ul>
        {/if}

        <!-- **The way back.** An update replaces the program in place and keeps the one it
             replaced, so this is the other half of that promise: the automatic rescue only catches
             a version that cannot start at all, and a version that starts, stays up and is simply
             worse needs a person to say so. Two steps, like removing the model, because it replaces
             the running program and there is no undo from here.

             Not in the `facts` list above, which is a mirror by construction — a control lives in
             its own `caps` list. -->
        {#if previousVersion && canInstall}
          <ul class="caps updates">
            <li>
              {#if goingBack}
                <span class="k">go back to {previousName}?</span>
                <span class="muted">
                  Formicaria will restart into {previousName}, which is the version you were using
                  before. Your notes are not touched. Getting {cfg.version} again means downloading it
                  again.
                </span>
                <button class="primary" onclick={goBack}>Yes, go back</button>
                <button onclick={() => (goingBack = false)}>Cancel</button>
              {:else}
                <span class="k">earlier version</span>
                <span class="muted">
                  {previousName} is still in this folder, in case {cfg.version} does not suit you.
                </span>
                <button onclick={() => (goingBack = true)}>Go back to {previousName}…</button>
              {/if}
            </li>
            {#if goBackError}
              <li><span class="bad">{goBackError}</span></li>
            {/if}
          </ul>
        {/if}
      </section>

      {#if cfg.env.length}
        <section data-cat="about">
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
    height: var(--app-h);
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
    min-width: 0;
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
    align-items: flex-start;
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
  /* **A button in these lists looks like a button** — unless it is a text link or a key chip, which
     have looks of their own. Until 2026-09-11 nothing styled them, so *Check now*, *Get*, *Install*,
     *Go back* and *Remove the model…* fell back to the web view's default drawing, and on the owner's
     phone *Check now* did not read as something to press. `BackupPanel`'s look, a size smaller for a
     row, so the two panels agree. */
  .caps button:not(.link):not(.binding) {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .caps button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  /* Pressed. A touch screen has no hover, so this is the only sign a tap landed before the answer. */
  .caps button:not(.link):not(.binding):not(.primary):active:not(:disabled) {
    background: var(--border);
  }
  .caps button.primary:active:not(:disabled) {
    filter: brightness(0.9);
  }
  .caps button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  @media (pointer: coarse) {
    .caps button:not(.link):not(.binding) {
      min-height: 2.5rem;
    }
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
  /* **The panel was never given a narrow layout at all.** Until 2026-09-08 this file held one
     `@media`, and it was `(pointer: coarse)` — touch-target sizing, nothing about width. A phone
     rendered the desktop layout, and the owner's complaint was exact: *"it is hard to understand
     which line corresponds to which option."*

     Measured at 390×844, the cause is `.k`'s `min-width: 9rem`: it reserves about 45% of a phone's
     content width for the label, so the value is squeezed into the rest and wraps — and since the
     item is inline flow rather than two columns, **the wrap lands at column 0, directly under the
     label**. `version` / `dev — built from source, not a release` read as two separate settings,
     one of them called "a release".

     Deliberately last in the sheet. `.k`, `.line`, `.assets input` and `.vault` are all defined
     *below* the file's first `@media`, at the same specificity as the overrides here, so a block
     placed up there silently loses to them — which is exactly what the first attempt did. Same
     breakpoint as `App.svelte`'s, which is where the app already decides it is on a phone. */
  @media (max-width: 40rem) {
    /* The label column goes away everywhere. `.caps` is the shared class of eight lists, only one
       of which is key/value; in the rest `.k` is a flex item sitting next to its own explanation
       ("Off" — "Not shared. Only this computer…"), and 9rem of reserved label was pushing that
       explanation into a three-line ribbon down the right-hand edge. */
    .k {
      min-width: 0;
    }
    /* With every option now two lines tall, 8px between them is not enough to read as a break. */
    .caps {
      gap: var(--space-3);
    }
    /* A choice row gets the same treatment as a fact: control and name on one line, the sentence
       explaining it on the next, indented past the control so it stays attached to the option
       above rather than the one below.

       `.choice .k`'s own `min-width: 5.5rem` is a third of a phone's content width held for the
       word "Off", which squeezed "Not shared. Only this computer can open your notes." into a
       ribbon down the right-hand edge. Dropping it alone is not enough and is actively worse: a
       flex item with `min-width: 0` shrinks past its own longest word, and "Audio transcription
       on" then printed *on top of* its description. Giving the description the whole next line is
       what makes the reset safe — `.k` no longer shares a row with anything that can squeeze it. */
    .choice {
      flex-wrap: wrap;
      /* `.choice`'s `gap` applies between the wrapped lines too, which put as much space between
         an option and its own description as between the description and the next option — the
         original complaint again, one level down. The list's own gap is what separates options. */
      row-gap: 0;
    }
    .choice .k {
      min-width: 0;
    }
    .choice input {
      flex: none;
    }
    .choice .muted {
      flex: 1 1 100%;
      padding-left: calc(1rem + var(--space-2));
    }
    /* Only the machine facts are key/value, and only they need stacking: label above, value
       indented under it, each fact bracketed by a rule. A value that wraps then stays visibly
       inside the option it belongs to instead of starting what looks like a new one. */
    .facts li {
      display: block;
      padding-left: var(--space-3);
      border-left: 2px solid var(--border);
    }
    .facts .k {
      display: block;
      font-weight: 600;
      color: var(--text);
    }
    /* A label and its control stop competing for one line: the control takes the next one whole.
       `.line` had no `flex-wrap` at all, so "Send attachments under" and its 6rem box squashed
       against each other. */
    .line {
      flex-wrap: wrap;
    }
    .assets input {
      width: 100%;
    }
    /* **Except a checkbox, which must never leave its own label.** The consent rows are also
       `.line`, and wrapping them put the box alone on one row with "Keep a record of what you
       change in AI suggestions" underneath — reading as a tick belonging to the paragraph above
       it. The two consent questions here are the ones where mis-reading which box is which has a
       consequence that cannot be taken back, so the span wraps *inside* the row instead, and the
       box aligns to its first line. */
    .consent {
      flex-wrap: nowrap;
      align-items: flex-start;
    }
    .consent input {
      flex: none;
    }
    /* `.vault` was `padding: var(--space-2) 0` — no separator of any kind, so with two vaults the
       path, the attachment box and the two consent questions of one ran straight into the next
       one's heading. */
    .vault {
      padding-left: var(--space-3);
      border-left: 2px solid var(--border);
    }
    /* The update rows stack the way the machine facts above them do: what is true on its own line in
       bold, the sentence under it, the button under that. Inline, the label ran straight into its
       sentence — "up to date Last looked today at 10:49" — which is how the owner quoted it back
       from the phone (2026-09-11). `> li >` leaves out the checkbox row, whose `.k` sits inside its
       label. */
    .updates > li > .k {
      display: block;
      font-weight: 600;
      color: var(--text);
    }
    .updates > li > .muted,
    .updates > li > .bad {
      display: block;
    }
    .updates > li > button {
      margin-top: var(--space-2);
    }
  }

  /* ============================================================================================
     The look of the window (2026-10-11). Same options, same order, same words: this is the frame,
     the cards and the controls. Kept in one block at the end so it reads as one decision.

     - **A header that stays**, with a way to close that is always in reach.
     - **A list of sections** beside the page on a wide screen and above it on a narrow one.
     - **Each section is a card**, so where one ends is something you can see.
     - **Controls drawn by the app**, not the browser: a switch for on/off, a ring for one-of,
       the house button for any button that had no look of its own.
     ============================================================================================ */
  .panel {
    width: min(58rem, 96vw);
    padding: 0 0 var(--space-5);
    display: grid;
    grid-template-columns: 12.5rem minmax(0, 1fr);
    align-content: start;
    gap: 0;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }
  .panel > :global(*),
  .panel > .cat > :global(*) {
    grid-column: 2;
    margin-inline: var(--space-5);
  }
  /* A wrapper that only says which category its sections are in. */
  .cat {
    display: contents;
  }
  .panel-head {
    grid-column: 1 / -1;
    position: sticky;
    top: 0;
    z-index: 3;
    align-items: center;
    margin: 0;
    padding: var(--space-3) var(--space-5);
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .find {
    flex: 1 1 10rem;
    min-width: 0;
    max-width: 20rem;
    margin-left: auto;
    box-sizing: border-box;
    padding: 0.4rem 0.7rem;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }
  .find:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  /* A search result says which category it lives in. */
  .panel :global(section[data-found])::before {
    content: attr(data-found);
    display: block;
    margin-bottom: var(--space-2);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .nothing {
    padding: var(--space-5) 0;
    text-align: center;
  }
  .t-cat {
    display: none;
  }
  /* Only on a narrow screen, where a category is a screen of its own. */
  .back {
    display: none;
    width: 2rem;
    height: 2rem;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--text-xl);
    line-height: 1;
    cursor: pointer;
  }
  /* The window of actions the ＋ opens: one column, no categories. */
  .panel.actions-only {
    width: min(38rem, 94vw);
    grid-template-columns: minmax(0, 1fr);
  }
  .panel.actions-only > :global(*),
  .panel.actions-only > .cat > :global(*) {
    grid-column: 1;
  }
  .panel.actions-only .group {
    display: none;
  }
  .panel.actions-only .x {
    margin-left: auto;
  }
  .x {
    margin-left: var(--space-2);
    width: 2rem;
    height: 2rem;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-md);
    cursor: pointer;
  }
  .x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .jump {
    grid-column: 1;
    grid-row: 2 / span 40;
    align-self: start;
    position: sticky;
    top: 3.4rem;
    z-index: 2;
    margin: 0;
    padding: var(--space-3) var(--space-2) var(--space-3) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: calc(100% - 3.4rem);
    box-sizing: border-box;
  }
  .jump button {
    display: flex;
    flex-direction: column;
    gap: 1px;
    text-align: left;
    padding: 0.45rem 0.65rem;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    min-width: 0;
  }
  .cat-name,
  .cat-state {
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cat-name {
    color: var(--text);
  }
  .cat-state {
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--text-subtle);
  }
  .jump button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .jump button.on {
    background: var(--accent-subtle);
    color: var(--text);
    font-weight: 600;
    box-shadow: inset 2px 0 0 var(--accent);
  }
  /* The two labels that divide the page: what you can change here, and what is only shown. */
  .group {
    margin-top: var(--space-5);
    margin-bottom: var(--space-2);
    padding: 0;
    border: none;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .group :global(.muted) {
    text-transform: none;
    letter-spacing: 0;
  }
  .panel :global(section) {
    margin-block: var(--space-2);
    padding: var(--space-4);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    scroll-margin-top: 4.2rem;
  }
  /* One size of section title, whichever component wrote it. */
  .panel :global(section > :is(h2, h3)) {
    margin: 0 0 var(--space-2);
    font-size: var(--text-base);
    font-weight: 650;
    letter-spacing: -0.005em;
    color: var(--text);
  }
  .panel :global(ul) {
    list-style: none;
    padding-left: 0;
  }
  .actions {
    margin-top: var(--space-3);
  }

  /* ---- Controls ---- */
  .panel :global(input[type='checkbox']) {
    appearance: none;
    flex: 0 0 auto;
    position: relative;
    width: 2.1rem;
    height: 1.2rem;
    margin: 0.1rem 0 0;
    border-radius: 999px;
    background: var(--border-strong);
    cursor: pointer;
    vertical-align: middle;
    transition: background var(--dur-fast) var(--ease);
  }
  .panel :global(input[type='checkbox'])::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(1.2rem - 4px);
    height: calc(1.2rem - 4px);
    border-radius: 50%;
    background: #fff;
    box-shadow: var(--shadow-sm);
    transition: transform var(--dur-fast) var(--ease);
  }
  .panel :global(input[type='checkbox']:checked) {
    background: var(--accent);
  }
  .panel :global(input[type='checkbox']:checked)::before {
    transform: translateX(0.9rem);
  }
  .panel :global(input[type='radio']) {
    appearance: none;
    flex: 0 0 auto;
    width: 1.05rem;
    height: 1.05rem;
    margin: 0.15rem 0 0;
    border: 2px solid var(--border-strong);
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
    vertical-align: middle;
  }
  .panel :global(input[type='radio']:checked) {
    border-color: var(--accent);
    background: radial-gradient(circle, var(--accent) 0 45%, transparent 50%);
  }
  .panel :global(input:is([type='checkbox'], [type='radio']):focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .panel :global(input:is([type='checkbox'], [type='radio']):disabled) {
    opacity: 0.5;
    cursor: default;
  }
  /* Any button that was left to the browser. One with a class has a look of its own already. */
  .panel :global(button:not([class])) {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-elevated);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .panel :global(button:not([class]):hover:not(:disabled)) {
    border-color: var(--accent);
  }
  .panel :global(button:not([class]):disabled) {
    opacity: 0.5;
    cursor: default;
  }

  /* ---- The sections other components write (appearance, calendars, mail) ----
     They use this panel's words for things (`caps`, `k`, `muted`, `choice`, `binding`, `group`) and,
     being separate components, never received its styles: their buttons were the browser's own.
     Said once here, for anything inside the panel. */
  .panel :global(section .muted) {
    font-size: var(--text-sm);
    line-height: 1.5;
    color: var(--text-muted);
  }
  .panel :global(section ul.caps) {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .panel :global(section .k) {
    color: var(--text-muted);
  }
  .panel :global(section label.choice) {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .panel :global(section p.group) {
    margin: var(--space-4) 0 var(--space-2);
    padding: 0;
    border: none;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .panel :global(button.binding) {
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-elevated);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .panel :global(button.binding:hover:not(:disabled)) {
    border-color: var(--accent);
  }
  .panel :global(button.binding[aria-pressed='true']) {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  .panel :global(button.binding:disabled) {
    opacity: 0.5;
    cursor: default;
  }
  .panel :global(section .row) {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .panel :global(input[type='color']) {
    width: 2rem;
    height: 1.6rem;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }
  /* Fields and the buttons these sections name by role (`primary`, `danger`, `link`). */
  .panel :global(:where(input:not([type]), input[type='text'], input[type='url']):not([class])),
  .panel :global(:where(input[type='password'], input[type='email']):not([class])),
  .panel :global(:where(select, textarea):not([class])) {
    box-sizing: border-box;
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }
  .panel :global(:where(input, select, textarea):not([class]):focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
    border-color: transparent;
  }
  .panel :global(section > :not(.caps) button:is(.primary, .danger)),
  .panel :global(section > button:is(.primary, .danger)) {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-elevated);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .panel :global(section button.primary) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
  }
  .panel :global(section button.danger) {
    border-color: var(--danger-bg);
    color: var(--danger-fg);
  }
  .panel :global(section button:is(.primary, .danger):disabled) {
    opacity: 0.5;
    cursor: default;
  }
  .panel :global(section button.link) {
    padding: 0;
    border: none;
    background: none;
    color: var(--link);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }

  /* The calendar and mail forms: their fields and buttons, wherever their own rules left them
     to the browser. */
  .panel :global(section :is(.add, .offset, .line) :is(input, select)) {
    box-sizing: border-box;
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }
  .panel :global(section :is(.add, .offset, .line) :is(input, select):focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .panel :global(section :is(.add, .offset, .line) button:not(.link)) {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-elevated);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .panel :global(section :is(.add, .offset, .line) button.primary) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
  }
  .panel :global(section :is(.add, .offset, .line) button:disabled) {
    opacity: 0.5;
    cursor: default;
  }

  /* Lists that follow one another inside a card are separate groups of rows. */
  .panel :global(section ul.caps + ul.caps) {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  /* ---- One sentence in view, the rest under *More* ----
     The explanation is all still there; it is closed until asked for. */
  .panel :global(.lede) {
    margin: 0 0 var(--space-2);
  }
  .panel :global(details.more) {
    margin: 0 0 var(--space-3);
    font-size: var(--text-sm);
  }
  .panel :global(details.more > summary) {
    display: inline-block;
    cursor: pointer;
    color: var(--link);
    list-style: none;
    user-select: none;
  }
  .panel :global(details.more > summary::-webkit-details-marker) {
    display: none;
  }
  .panel :global(details.more > summary)::after {
    content: ' ›';
  }
  .panel :global(details.more[open] > summary)::after {
    content: ' ⌄';
  }
  .panel :global(details.more > p) {
    margin: var(--space-2) 0 0;
  }

  /* ---- Rows ----
     A row is a name and what is true of it. The name hangs in its own column, so a long answer
     wraps under the answer and not under the name; a faint line separates one row from the next. */
  .panel :global(.caps li + li) {
    padding-top: var(--space-2);
    border-top: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }
  .panel :global(.caps li:has(> .k:first-child)) {
    position: relative;
    padding-left: 11rem;
    line-height: 1.5;
  }
  .panel :global(.caps li > .k:first-child) {
    position: absolute;
    left: 0;
    width: 10.5rem;
    min-width: 0;
  }
  .caps li > :global(button) {
    margin-left: var(--space-2);
  }
  /* A switch row: the switch, its state in a word, then what it does. */
  .choice {
    gap: var(--space-3);
  }
  .panel :global(.choice .k) {
    min-width: 4.2rem;
    color: var(--text);
    font-weight: 600;
  }
  .panel :global(.choice .muted) {
    margin: 0;
    flex: 1 1 0;
    min-width: 0;
  }

  /* ---- A vault's block ---- */
  .vault {
    padding: var(--space-3) 0;
  }
  .vault + .vault {
    border-top: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }
  .vault .line {
    font-size: var(--text-sm);
    gap: var(--space-3);
  }
  .vault .line:first-child {
    font-size: var(--text-base);
  }
  .vault label.line {
    margin-block: var(--space-2);
  }

  /* A narrow screen: the list of sections lies across the top and scrolls sideways. */
  @media (max-width: 46rem) {
    /* A plain stack, not a one-column grid: a sticky row inside a grid stays within its own
       cell, so the list of sections scrolled away with the first screenful. */
    .panel {
      display: block;
      width: 100%;
    }
    .panel > :global(*),
    .panel > .cat > :global(*) {
      grid-column: 1;
      margin-inline: var(--space-3);
    }
    .panel-head {
      padding-inline: var(--space-3);
    }
    /* A switch, its word, and then the explanation on a line of its own. */
    .panel :global(.choice) {
      flex-wrap: wrap;
    }
    .panel :global(.choice .muted) {
      flex-basis: 100%;
    }
    /* The list of categories is the first screen; a category is the second. */
    .jump {
      position: static;
      max-height: none;
      padding: var(--space-3);
      gap: var(--space-2);
    }
    .jump button {
      padding: 0.8rem 0.9rem;
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      background: var(--surface);
      font-size: var(--text-base);
    }
    .jump button.on {
      box-shadow: none;
      background: var(--surface);
      font-weight: 400;
    }
    .panel:not(.at-list) .jump {
      display: none;
    }
    .panel.at-list :global(section),
    .panel.at-list :global(p.group),
    .panel.at-list .actions {
      display: none;
    }
    .panel:not(.at-list):not(.actions-only) .back {
      display: grid;
      place-items: center;
    }
    /* An open category: its name, the way back and the way out. Searching is done from the list,
       and while something is typed the field has the header to itself. */
    .panel:not(.at-list):not(.searching) .find {
      display: none;
    }
    .panel:not(.at-list):not(.searching) .t-all,
    .panel.searching h2 {
      display: none;
    }
    .panel:not(.at-list):not(.searching) .t-cat {
      display: inline;
    }
    .panel:not(.at-list):not(.searching) .x {
      margin-left: auto;
    }
    .panel.searching .find {
      max-width: none;
      margin-left: 0;
    }
    .panel :global(section) {
      padding: var(--space-3);
      scroll-margin-top: 4rem;
    }
    /* No room for two columns: the name sits above its answer. */
    .panel :global(.caps li:has(> .k:first-child)) {
      padding-left: 0;
    }
    .panel :global(.caps li > .k:first-child) {
      position: static;
      display: block;
      width: auto;
      font-size: var(--text-xs);
    }
  }
  /* The Vaults heading carries the one way in to making a vault, where the vaults are listed. */
  .vaults-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .new-vault {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--fg);
    padding: 4px 10px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
  /* The move form sits under the folder it moves, framed as one step. */
  .move {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
  }
  .move input {
    flex: 1;
    min-width: 0;
  }
</style>

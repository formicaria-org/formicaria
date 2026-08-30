<script lang="ts">
  /// **Two ways into one file.**
  ///
  /// `MASTERPLAN.md` has always said extensibility comes from *"CSS themes and declarative `.view`
  /// files — no code execution"*, and `app.css` was built as three layers precisely so *"a theme is
  /// one file"*. The file half is `crates/fm-app/src/themes.rs`. This is the authoring half, and it
  /// has to serve someone who works only through these screens and never opens a text editor.
  ///
  /// So: a **form** of three questions that writes token declarations, and a **text box** holding
  /// the same file underneath for when three questions run out. One model, two views — the shape
  /// VS Code settles on, and the only one that serves both without two mechanisms fighting over
  /// which is authoritative.
  ///
  /// **The form is a list, not a language.** A token may be re-valued; a new token cannot be
  /// invented from here — the same line `keys.ts` draws when it lets you rebind a command but not
  /// invent one. The moment this wants a selector or a media query it has become the query builder
  /// this project has twice refused, and the text box below already exists for exactly that.
  ///
  /// When the file contains more than the form can say, the form goes read-only and **refuses to
  /// rewrite it** rather than flattening it — the rule `views::save_view` already applies to a
  /// filter richer than one tag, for the same reason.
  import { listThemes, readTheme, saveTheme, deleteTheme, renameTheme } from './ipc';
  import * as appearance from './appearance';
  import type { ThemeInfo } from './types';

  interface Props {
    /// The theme in use, or `null`. Owned by `App.svelte`, which also applies it.
    selected: appearance.Selection | null;
    /// Switch to a theme, or to none. The caller persists and applies.
    onselect: (sel: appearance.Selection | null) => void;
  }
  let { selected, onselect }: Props = $props();

  /// The file the form writes. One per vault, created on first use — the form is "how this app
  /// looks to me", not a theme-authoring studio, so it does not ask for a name.
  const MINE = 'My appearance';

  let themes = $state<ThemeInfo[]>([]);
  let prefs = $state<appearance.AppearancePrefs>({});
  /// The selected file carries something the three questions cannot express, so the form must not
  /// rewrite it.
  let handWritten = $state(false);
  let busy = $state(false);
  let problem = $state('');

  /// The text box, and what it is editing. `draft` is applied live; nothing reaches the vault until
  /// **Keep**, which is the layer of the escape hatch that catches almost everything.
  let editing = $state<string | null>(null);
  let draft = $state('');
  let before = $state('');

  const FONTS: [string, string][] = [
    ['System', "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif"],
    ['Serif', "ui-serif, Georgia, 'Times New Roman', serif"],
    ['Monospace', 'ui-monospace, SFMono-Regular, Menlo, monospace'],
  ];
  const SIZES: [string, string][] = [
    ['Small', '0.875rem'],
    ['Normal', '0.9375rem'],
    ['Large', '1.0625rem'],
  ];
  const ACCENTS = ['#dc2626', '#2563eb', '#059669', '#7c3aed'];

  async function refresh() {
    try {
      themes = await listThemes();
      problem = '';
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => void refresh());

  /// Load the selected theme into the form, so the controls show what is actually in force rather
  /// than defaults that quietly disagree with the screen.
  $effect(() => {
    const sel = selected;
    if (!sel) {
      prefs = {};
      handWritten = false;
      return;
    }
    void readTheme(sel.name, sel.vault)
      .then((css) => {
        const read = appearance.fromCss(css);
        prefs = read.prefs;
        handWritten = read.extra;
      })
      .catch(() => {
        prefs = {};
        handWritten = false;
      });
  });

  /// Write the form's answers. Creates the file on first use and switches to it, so a first-time
  /// user picks a colour and sees it — rather than picking a colour and being told to make a theme.
  async function put(next: appearance.AppearancePrefs) {
    if (handWritten) return;
    prefs = next;
    busy = true;
    try {
      const target = selected ?? { vault: themes[0]?.vault ?? '', name: MINE };
      themes = await saveTheme(target.name, appearance.toCss(next), target.vault);
      problem = '';
      if (!selected) {
        const made = themes.find((t) => t.name === 'my-appearance') ?? themes[0];
        if (made) onselect({ vault: made.vault ?? '', name: made.name });
      }
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function openEditor(t: ThemeInfo) {
    try {
      before = await readTheme(t.name, t.vault ?? '');
      draft = before;
      editing = t.name;
      problem = '';
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    }
  }

  /// Live preview. Typing restyles the app immediately, which is the fastest possible way to learn
  /// that a rule was a bad idea — and **Cancel** puts back exactly what was there.
  function onDraft(v: string) {
    draft = v;
    appearance.apply(v);
  }

  async function keep() {
    const name = editing;
    if (name === null) return;
    busy = true;
    try {
      const t = themes.find((x) => x.name === name);
      themes = await saveTheme(name, draft, t?.vault ?? '');
      editing = null;
      problem = '';
      // Wearing what you just wrote. Anything else means editing a theme and seeing no change.
      if (t) onselect({ vault: t.vault ?? '', name });
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function cancel() {
    // Put back what was on screen before the preview, whether that was another theme or nothing.
    if (selected?.name === editing) appearance.apply(before);
    else if (!selected) appearance.clear();
    editing = null;
  }

  /// Renaming happens in place in the list rather than in a dialog: it is a rare action on a thing
  /// already on screen, and this app has decided more than once that a rare action does not earn a
  /// new surface. Enter commits, Escape abandons.
  let renaming = $state<string | null>(null);
  let newName = $state('');

  function startRename(t: ThemeInfo) {
    renaming = t.name;
    newName = t.name;
  }

  async function commitRename(t: ThemeInfo) {
    const to = newName.trim();
    renaming = null;
    if (!to || to === t.name) return;
    busy = true;
    try {
      themes = await renameTheme(t.name, to, t.vault ?? '');
      problem = '';
      // Follow it if it was the one being worn, or the selection points at a file that has moved
      // and the next launch falls back to the built-in look for no visible reason.
      if (selected?.name === t.name) onselect({ vault: t.vault ?? '', name: to });
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function remove(t: ThemeInfo) {
    busy = true;
    try {
      if (selected?.name === t.name) onselect(null);
      themes = await deleteTheme(t.name, t.vault ?? '');
      problem = '';
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <h3>Appearance</h3>
  <p class="muted">
    Colours and type. Unlike the settings around it, this one <strong>is kept in your vault</strong>,
    so it travels with your notes — but which one is switched on stays on this device, the same as
    dark and light.
  </p>

  {#if handWritten}
    <p class="muted">
      “{selected?.name}” has more in it than these controls can describe, so they are switched off
      rather than overwriting what you wrote. Use <strong>Edit</strong> below.
    </p>
  {/if}

  <ul class="caps">
    <li>
      <span class="k">Accent</span>
      <span class="row">
        {#each ACCENTS as c (c)}
          <button
            class="swatch"
            style="background:{c}"
            disabled={handWritten || busy}
            aria-label="accent {c}"
            aria-pressed={prefs.accent === c}
            onclick={() => put({ ...prefs, accent: c })}></button>
        {/each}
        <input
          type="color"
          aria-label="a colour of your own"
          disabled={handWritten || busy}
          value={prefs.accent ?? '#dc2626'}
          onchange={(e) => put({ ...prefs, accent: e.currentTarget.value })} />
      </span>
    </li>
    <li>
      <span class="k">Text size</span>
      <span class="row">
        {#each SIZES as [label, value] (value)}
          <button
            class="binding"
            disabled={handWritten || busy}
            aria-pressed={prefs.textBase === value}
            onclick={() => put({ ...prefs, textBase: value })}>{label}</button>
        {/each}
      </span>
    </li>
    <li>
      <span class="k">Font</span>
      <span class="row">
        {#each FONTS as [label, stack] (label)}
          <button
            class="binding"
            disabled={handWritten || busy}
            aria-pressed={prefs.fontSans === stack}
            onclick={() => put({ ...prefs, fontSans: stack })}>{label}</button>
        {/each}
      </span>
    </li>
  </ul>

  <p class="group">Your own themes</p>
  <ul class="caps">
    <li>
      <label class="choice">
        <input type="radio" name="theme-file" checked={!selected} onchange={() => onselect(null)} />
        <span class="k">None</span>
        <span class="muted">The look this app ships with.</span>
      </label>
    </li>
    {#each themes as t (`${t.vault}/${t.name}`)}
      <li>
        <label class="choice">
          <input
            type="radio"
            name="theme-file"
            disabled={!!t.error}
            checked={selected?.name === t.name && selected?.vault === (t.vault ?? '')}
            onchange={() => onselect({ vault: t.vault ?? '', name: t.name })} />
          {#if renaming === t.name}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="rename"
              autofocus
              aria-label="new name for {t.name}"
              bind:value={newName}
              onkeydown={(e) => {
                if (e.key === 'Enter') commitRename(t);
                else if (e.key === 'Escape') renaming = null;
              }}
              onblur={() => commitRename(t)} />
          {:else}
            <span class="k">{t.name}</span>
          {/if}
          {#if t.error}
            <span class="muted">{t.error}</span>
          {:else}
            <span class="muted">{Math.max(1, Math.round(t.bytes / 1024))} KB</span>
          {/if}
        </label>
        <span class="row">
          <button class="binding" disabled={busy} onclick={() => openEditor(t)}>Edit</button>
          <button class="binding" disabled={busy} onclick={() => startRename(t)}>Rename</button>
          <button class="binding" disabled={busy} onclick={() => remove(t)}>Delete</button>
        </span>
      </li>
    {/each}
  </ul>

  {#if editing !== null}
    <!-- Tier 2. The same file the form above writes, in full. Applied as you type and not written
         anywhere until Keep, so trying something is free and abandoning it costs nothing. -->
    <div class="editor">
      <label class="ed-label" for="theme-css">
        “{editing}” — applied as you type. Nothing is saved until you keep it.
      </label>
      <textarea
        id="theme-css"
        class="theme-css"
        spellcheck="false"
        rows="12"
        value={draft}
        oninput={(e) => onDraft(e.currentTarget.value)}></textarea>
      <p class="muted">
        Set any of the app's own names — <code>--bg</code>, <code>--surface</code>,
        <code>--text</code>, <code>--accent</code>, <code>--font-sans</code>,
        <code>--radius-md</code>, <code>--space-4</code> and the rest ({appearance.SUPPORTED.length}
        in all). Those names are a promise and will not move under you; anything else you target
        inside the app is not, and may change.
      </p>
      <span class="row">
        <button class="binding" disabled={busy} onclick={keep}>Keep</button>
        <button class="binding" onclick={cancel}>Cancel</button>
      </span>
    </div>
  {/if}

  {#if problem}<p class="muted">{problem}</p>{/if}
</section>

<style>
  .row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    flex-wrap: wrap;
  }
  .swatch {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-pill);
    border: 2px solid var(--border-strong);
    cursor: pointer;
  }
  .swatch[aria-pressed='true'] {
    border-color: var(--text);
  }
  .rename {
    font: inherit;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    min-width: 8rem;
  }
  .editor {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .ed-label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .theme-css {
    width: 100%;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    resize: vertical;
  }
</style>

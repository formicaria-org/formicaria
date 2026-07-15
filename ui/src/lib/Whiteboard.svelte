<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // A "board" note: its body IS an Excalidraw scene (JSON). The whole editor —
  // React, ReactDOM, and Excalidraw itself — is imported LAZILY here, so it lands
  // in its own code-split chunk and only downloads when a board is actually
  // opened; the base bundle is untouched (same pattern as KaTeX/Mermaid in
  // render.ts). Excalidraw is a React component, so we mount a React root into
  // this Svelte host via `createElement` (no JSX → no extra build plugin).
  let {
    body = '',
    theme = 'dark',
    onSave,
  }: { body: string; theme?: 'dark' | 'light'; onSave: (json: string) => void } = $props();

  let host = $state<HTMLDivElement | undefined>(undefined);
  // React root + a pending-save buffer. `any` because the React/Excalidraw types
  // aren't in scope until the lazy import resolves.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let root: any = null;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let pending: string | null = null; // latest serialized scene not yet persisted
  let lastSerialized = '';
  let loadError = $state<string | null>(null);

  // Excalidraw's initialData wants a plain object; a board that isn't valid JSON
  // (or is empty) opens as a blank canvas rather than erroring.
  function parseInitial(b: string): Record<string, unknown> {
    try {
      const d = JSON.parse(b);
      if (d && Array.isArray(d.elements)) {
        // `collaborators` must be a Map on restore; drop it (we're single-user).
        const appState = d.appState ? { ...d.appState, collaborators: undefined } : undefined;
        return { elements: d.elements, appState, files: d.files, scrollToContent: true };
      }
    } catch {
      /* not JSON — blank canvas */
    }
    return {};
  }

  function flush() {
    if (pending !== null) {
      const json = pending;
      pending = null;
      onSave(json);
    }
  }

  onMount(async () => {
    try {
      const React = await import('react');
      const { createRoot } = await import('react-dom/client');
      const excal = await import('@excalidraw/excalidraw');
      await import('@excalidraw/excalidraw/index.css');
      const { Excalidraw, serializeAsJSON } = excal;

      lastSerialized = body;
      const initialData = parseInitial(body);

      // Excalidraw fires onChange on every pointer move; serialize + debounce the
      // write so we don't hammer update_body, and skip no-op saves.
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const onChange = (elements: any, appState: any, files: any) => {
        const json = serializeAsJSON(elements, appState, files, 'local');
        if (json === lastSerialized) return;
        lastSerialized = json;
        pending = json;
        clearTimeout(saveTimer);
        saveTimer = setTimeout(flush, 600);
      };

      if (!host) return;
      root = createRoot(host);
      root.render(
        React.createElement(Excalidraw, {
          initialData,
          onChange,
          theme,
          // The app owns the theme; hide Excalidraw's own dark/light toggle.
          UIOptions: { canvasActions: { toggleTheme: false } },
        }),
      );
    } catch (e) {
      loadError = String(e);
    }
  });

  onDestroy(() => {
    clearTimeout(saveTimer);
    flush(); // persist any change still inside the debounce window
    root?.unmount?.();
  });
</script>

<div class="board-host" bind:this={host}>
  {#if loadError}<p class="board-error">Couldn't load the board editor: {loadError}</p>{/if}
</div>

<style>
  .board-host {
    position: relative;
    flex: 1;
    min-height: 0;
    width: 100%;
  }
  .board-error {
    padding: 1rem;
    color: var(--muted);
  }
  /* Excalidraw fills whatever box it's given. */
  :global(.board-host .excalidraw) {
    height: 100%;
  }
</style>

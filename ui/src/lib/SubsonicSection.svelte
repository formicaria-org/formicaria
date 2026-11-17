<script lang="ts">
  // **Your music in other players** (`decisions.md` 2026-11-17; `fm-serve/src/subsonic.rs`): any
  // Subsonic player (Symfonium, DSub, Tempo, Feishin…) streams Music from this computer's own files.
  // Off until turned on; its own password; reachable from a phone only while sharing is on.
  import { onMount } from 'svelte';
  import { subsonic, subsonicSet, type Subsonic } from './ipc';

  let st = $state<Subsonic | null>(null);
  let shown = $state(false);
  let said = $state('');
  onMount(() => {
    void subsonic()
      .then((s) => (st = s))
      .catch(() => (st = null));
  });
  async function set(change: { on?: boolean; renew?: boolean }) {
    said = '';
    try {
      st = await subsonicSet(change);
    } catch (e) {
      said = String(e);
    }
  }
</script>

{#if st}
  <section data-words="subsonic music player symfonium dsub stream songs playlists">
    <h3>Your music in other players</h3>
    <p class="muted">
      Any Subsonic player (Symfonium, DSub, Tempo, Feishin…) can stream your Music — songs, albums,
      artists and playlists — from this computer's own files. Nothing is copied.
    </p>
    <ul class="caps">
      <li>
        <span class="k">{st.on ? 'On' : 'Off'}</span>
        <button class="binding" onclick={() => void set({ on: !st!.on })}
          >{st.on ? 'Turn it off' : 'Turn it on'}</button
        >
      </li>
      {#if st.on}
        <li><span class="k">Server</span> <code>{location.origin}</code></li>
        <li><span class="k">User</span> <code>{st.user}</code></li>
        <li>
          <span class="k">Password</span>
          <code>{shown ? st.password : '••••••••••••'}</code>
          <button class="binding" onclick={() => (shown = !shown)}>{shown ? 'Hide' : 'Show'}</button
          >
          <button class="binding" onclick={() => void set({ renew: true })}>New password</button>
        </li>
      {/if}
    </ul>
    {#if st.on && !st.shared}
      <p class="muted">
        A player on another device reaches this computer only while <strong
          >sharing with a nearby device</strong
        > is on (above); then use this computer's address on your network.
      </p>
    {/if}
    {#if said}<p class="problem">{said}</p>{/if}
  </section>
{/if}

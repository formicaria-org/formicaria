<script lang="ts">
  // The one place a note's vault is shown to the user — reused by every renderer and the open
  // note, so "which audience is this?" looks identical everywhere. A dedicated, name-derived
  // colour plus the vault name; `dot` is the compact form for tight spots (calendar bars), where
  // the name rides in the tooltip. Renders nothing when there's no vault (a single-vault install
  // has no boundary to show). The colour tells the truth about who can see the note because the
  // vault is derived from where the file lives — never from anything the file says.
  import { hashHue } from './vaultColor';
  import { labelFor } from './vaults.svelte';

  let { vault, dot = false }: { vault?: string | null; dot?: boolean } = $props();
  /// **Callers keep passing the vault *name*** — the identity — and the label is resolved here, so the
  /// twenty-odd usages of this component did not each have to learn about labels. See
  /// `vaults.svelte.ts` for why the two differ.
  const shown = $derived(labelFor(vault));
  /// Keyed on what is *displayed*, deliberately: the same repository then gets the same colour on the
  /// laptop and on the phone, which is the whole point — a local folder name would give one audience
  /// two colours.
  const hue = $derived(shown ? hashHue(shown) : 0);
</script>

{#if vault}
  {#if dot}
    <span
      class="vault-dot"
      style="--vh:{hue}"
      title={`Vault: ${shown}`}
      aria-label={`Vault: ${shown}`}
    ></span>
  {:else}
    <span class="vault-badge" style="--vh:{hue}" title={`Vault: ${shown}`}>{shown}</span>
  {/if}
{/if}

<style>
  /* A vivid, name-keyed chip. Fixed low lightness so white text is legible on every hue, and so
     the chip reads the same over a light or a dark surface — the vault's identity, not a tag. */
  .vault-badge {
    display: inline-flex;
    align-items: center;
    max-width: 10rem;
    padding: 0.03rem 0.4rem;
    border-radius: var(--radius-sm, 5px);
    background: hsl(var(--vh) 52% 40%);
    border: 1px solid hsl(var(--vh) 52% 30%);
    color: #fff;
    font-size: 0.66rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    line-height: 1.5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .vault-dot {
    display: inline-block;
    flex: none;
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background: hsl(var(--vh) 55% 45%);
    box-shadow: 0 0 0 1px hsl(var(--vh) 55% 28%);
  }
</style>

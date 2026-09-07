<script lang="ts">
  // **Where a merge kept both sides of a disagreement, and the only thing that makes that
  // honest.**
  //
  // When two devices set the same field differently, `merge.rs` takes one by a stated rule and
  // writes the other into a `conflict-<field>` key beside it. `decisions.md` (2026-09-07) argues
  // at length that this is not last-write-wins — *"the loser is a line in the file, one the user
  // can read, grep, query, group a board by, and promote back in a tap"* — and then attaches a
  // condition to itself: **if the demotion is never shown, it becomes the fiat the entry denies.**
  // This panel is that condition. It is not decoration.
  //
  // Both actions go through `set_property`, the same path a person typing the value takes, so
  // nothing here is a second way to write a property:
  //
  //   - **Keep this one** sets the field to the other device's value *and* clears the record. The
  //     merge would drop it on its own — a demoted value equal to the winner is not a
  //     disagreement — but only at the next merge, and the user asked for it now.
  //   - **Dismiss** clears the record alone. That sticks: `merge_demoted` honours a removal by a
  //     side that had it, precisely so this button is not undone by the next pull.
  import { setProperty, type DemotedField } from './ipc';

  let {
    rows,
    onclose,
    onchanged,
  }: { rows: DemotedField[]; onclose: () => void; onchanged: () => void } = $props();

  // Per row, because one write failing says nothing about the others.
  let failed = $state<Record<string, string>>({});
  let busy = $state<string | null>(null);

  const key = (d: DemotedField, other: string) => `${d.id}\0${d.field}\0${other}`;

  // The frontmatter key holding the losers for this field — one spelling, shared with the
  // backend's `merge::DEMOTED_PREFIX`, because two would eventually disagree.
  const recordKey = (d: DemotedField) => `conflict-${d.field}`;

  async function act(d: DemotedField, other: string, promote: boolean) {
    const k = key(d, other);
    busy = k;
    delete failed[k];
    try {
      if (promote) await setProperty(d.id, d.field, other);
      await setProperty(d.id, recordKey(d), '');
      onchanged();
    } catch (e) {
      failed[k] = String(e);
    } finally {
      busy = null;
    }
  }

  const byVault = $derived([
    ...rows.reduce(
      (m, d) => m.set(d.vault, [...(m.get(d.vault) ?? []), d]),
      new Map<string, DemotedField[]>(),
    ),
  ]);

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="demoted-overlay">
  <button class="demoted-backdrop" aria-label="close" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="fields the two devices disagreed about">
    <div class="panel-head">
      <h2>Both devices had an answer</h2>
    </div>

    <p class="lede">
      These notes were edited in two places at once, and the two devices set the same field
      differently. Nothing was thrown away: the note shows one value and the other is kept beside it
      in the file. Pick one, or leave it — nothing here is blocking anything.
    </p>

    {#each byVault as [vault, items] (vault)}
      <div class="vault">
        <h3>{vault}</h3>
        {#each items as d (d.id + d.field)}
          <div class="note">
            <div class="title">{d.title || d.id}</div>
            <div class="field">
              <code>{d.field}</code> is <strong>{d.kept}</strong> here
            </div>
            {#each d.other as other (other)}
              <div class="line">
                <span class="other">the other device said <strong>{other}</strong></span>
                <div class="row-actions">
                  <button
                    class="primary"
                    disabled={busy === key(d, other)}
                    onclick={() => act(d, other, true)}
                  >
                    Keep “{other}”
                  </button>
                  <button
                    class="ghost"
                    disabled={busy === key(d, other)}
                    onclick={() => act(d, other, false)}
                    title="Keep “{d.kept}” and stop showing this"
                  >
                    Dismiss
                  </button>
                </div>
              </div>
              {#if failed[key(d, other)]}
                <p class="failed">{failed[key(d, other)]}</p>
              {/if}
            {/each}
          </div>
        {/each}
      </div>
    {/each}

    <p class="hint">
      Dismissing keeps the value the note already shows and removes the record of the disagreement.
      That sticks — the other device will not put it back.
    </p>

    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .demoted-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    /* Longhands, never the `padding` shorthand — a narrower rule added later would reset all four
       sides and discard the insets. Same arithmetic as every other overlay here. */
    box-sizing: border-box;
    height: 100dvh;
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
    padding-bottom: var(--safe-bottom);
    z-index: 80;
  }
  @media (pointer: coarse) {
    .demoted-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .demoted-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(34rem, 92vw);
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
    justify-content: space-between;
    gap: 12px;
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
  .lede,
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }
  .vault {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .note {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }
  .title {
    color: var(--text);
    font-weight: 600;
  }
  .field,
  .other {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .row-actions {
    display: flex;
    gap: var(--space-2);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger, #b00);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>

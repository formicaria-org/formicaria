<script lang="ts">
  // **Everything the merge settled on the user's behalf, in one place.**
  //
  // Two shapes, one category: *the two devices disagreed, the merge chose, and nothing is
  // blocked*. They were separate surfaces for about six hours (2026-09-07) and the second one
  // would have been the sixth chip in a toolbar this repo has already measured wrapping to four
  // rows on a phone (`App.svelte`, the 2026-07-19 device note). So they share a chip and a panel,
  // and each keeps its own section, its own sentence and its own action.
  //
  // When two devices set the same field differently, `merge.rs` takes one by a stated rule and
  // writes the other into a `conflict-<field>` key beside it. `decisions.md` (2026-09-07) argues
  // at length that this is not last-write-wins — *"the loser is a line in the file, one the user
  // can read, grep, query, group a board by, and promote back in a tap"* — and then attaches a
  // condition to itself: **if the demotion is never shown, it becomes the fiat the entry denies.**
  // This panel is that condition. It is not decoration.
  //
  // **A divergent field** goes through `set_property`, the same path a person typing the value
  // takes, so nothing here is a second way to write a property:
  //
  //   - **Keep this one** sets the field to the other device's value *and* clears the record. The
  //     merge would drop it on its own — a demoted value equal to the winner is not a
  //     disagreement — but only at the next merge, and the user asked for it now.
  //   - **Dismiss** clears the record alone. That sticks: `merge_demoted` honours a removal by a
  //     side that had it, precisely so this button is not undone by the next pull.
  //
  // **A note that came back** is the other shape, and its two answers are asymmetric:
  //
  //   - **Delete it again** is the ordinary `delete` command — no new write path — and it is
  //     *armed first*, because deleting a note is irreversible from a phone with no shell and
  //     every neighbouring destructive control in this app arms before it fires (`NotePanel`'s
  //     confirm strip, `BackupPanel`'s Remove/Cancel). It needs no record: the note is gone at
  //     HEAD on every device, so the row cannot come back.
  //   - **I have seen these** moves a per-device watermark. Per device on purpose — the two
  //     devices are not asking the same question. On the one that kept the note it is *"you edited
  //     this, they deleted it"*; on the one that deleted it, which fast-forwards onto the merge and
  //     never runs a keep of its own, it is *"you deleted this and it is back"*. Both people are
  //     owed an answer, so acknowledging on one must not silence the other.
  import { setProperty, deleteNote, keptSeen, type DemotedField, type KeptNote } from './ipc';

  let {
    rows,
    kept,
    onclose,
    onchanged,
  }: {
    rows: DemotedField[];
    kept: KeptNote[];
    onclose: () => void;
    onchanged: () => void;
  } = $props();

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

  // Armed, not fired: the id whose delete is one more tap away.
  let arming = $state<string | null>(null);

  async function removeAgain(k: KeptNote) {
    if (!k.id) return;
    busy = k.path;
    delete failed[k.path];
    try {
      await deleteNote(k.id);
      arming = null;
      onchanged();
    } catch (e) {
      failed[k.path] = String(e);
    } finally {
      busy = null;
    }
  }

  async function acknowledge() {
    busy = 'seen';
    try {
      await keptSeen();
      onchanged();
    } catch (e) {
      failed.seen = String(e);
    } finally {
      busy = null;
    }
  }

  const keptByVault = $derived([
    ...kept.reduce(
      (m, k) => m.set(k.vault, [...(m.get(k.vault) ?? []), k]),
      new Map<string, KeptNote[]>(),
    ),
  ]);

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

<div class="kept-overlay">
  <button class="kept-backdrop" aria-label="close" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="what the two devices disagreed about">
    <div class="panel-head">
      <h2>Both devices had an answer</h2>
    </div>

    <p class="lede">
      These notes were edited in two places at once. Nothing here is blocking anything — the choices
      below were already made for you, and you can change any of them.
    </p>

    {#if kept.length}
      <section class="section">
        <h3 class="section-head">Notes that came back</h3>
        <p class="lede">
          Deleted on one device while this one was still editing them. There was no text to merge,
          so the note was kept rather than lost — a note that comes back can be deleted again, and
          one deleted by the app cannot be brought back.
        </p>
        {#each keptByVault as [vault, items] (vault)}
          <div class="vault">
            <h3>{vault}</h3>
            {#each items as k (k.path)}
              <div class="note">
                <div class="title">{k.title || k.path}</div>
                <div class="line">
                  <span class="other">
                    {#if k.id}came back after the other device deleted it{:else}
                      a file, not a note — it came back too
                    {/if}
                  </span>
                  {#if k.id}
                    <div class="row-actions">
                      {#if arming === k.path}
                        <button
                          class="danger"
                          disabled={busy === k.path}
                          onclick={() => removeAgain(k)}
                        >
                          Yes, delete it
                        </button>
                        <button class="ghost" onclick={() => (arming = null)}>Cancel</button>
                      {:else}
                        <button class="ghost" onclick={() => (arming = k.path)}>
                          Delete it again
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
                {#if failed[k.path]}
                  <p class="failed">{failed[k.path]}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/each}
        <div class="actions">
          <button disabled={busy === 'seen'} onclick={acknowledge}> I have seen these </button>
        </div>
        {#if failed.seen}
          <p class="failed">{failed.seen}</p>
        {/if}
      </section>
    {/if}

    {#if rows.length}
      <section class="section">
        <h3 class="section-head">Both answers kept</h3>
        <p class="lede">
          The two devices set the same field differently. Nothing was thrown away: the note shows
          one value and the other is kept beside it in the file.
        </p>
      </section>
    {/if}

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

    {#if rows.length}
      <p class="hint">
        Dismissing keeps the value the note already shows and removes the record of the
        disagreement. That sticks — the other device will not put it back.
      </p>
    {/if}

    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .kept-overlay {
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
    .kept-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .kept-backdrop {
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
  /* Two shapes in one panel, so each needs a visible boundary or the reader cannot tell which
     sentence belongs to which set of buttons. */
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .section-head {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text);
    font-weight: 700;
  }
  /* The one destructive control here. It only ever appears already armed — the first tap swaps
     it in — so its colour is a confirmation of what the second tap does, not an invitation. */
  .danger {
    color: var(--danger, #b00);
    font-weight: 600;
  }
</style>

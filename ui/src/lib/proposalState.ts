// How a proposal ended, from its note alone — the UI's copy of `thread::proposal_state` in
// `crates/fm-app/src/thread.rs` (`decisions.md#git`, 2026-10-24, *a proposal is open, merged,
// declined or closed*). The note carries it, so every device and every collaborator reads the same.

export type ProposalState = 'open' | 'merged' | 'declined' | 'closed';

type WithProps = { props?: Record<string, unknown> | null };

const set = (v: unknown) =>
  v === true || (typeof v === 'string' && v.trim() !== '' && v !== 'false');

export function proposalState(p: WithProps): ProposalState {
  const props = p.props ?? {};
  if (set(props.declined)) return 'declined';
  if (set(props.accepted)) return 'merged';
  if (set(props.closed)) return 'closed';
  return 'open';
}

/** When it closed, as stamped (RFC3339): accepted, or declined. `null` while open or unrecorded. */
export function closedAt(p: WithProps): string | null {
  const props = p.props ?? {};
  const v = set(props.declined) ? props.declined_at : set(props.accepted) ? props.accepted : null;
  return typeof v === 'string' && v ? v : null;
}

/** The word a person reads for each state. */
export const STATE_LABEL: Record<ProposalState, string> = {
  open: 'Open',
  merged: 'Merged',
  declined: 'Declined',
  closed: 'Closed',
};

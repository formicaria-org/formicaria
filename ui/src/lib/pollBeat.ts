/// How often an open discussion should ask the backend what the agent is doing.
///
/// **Its own module so it can be tested without a clock.** The rule is two numbers and a branch;
/// proving it through the component costs seconds of real sleeping per assertion, and a test that
/// sleeps is a test someone eventually deletes from the CI gate. The component keeps the wiring
/// (a self-rescheduling timeout); this keeps the policy.

/// While an agent turn is in flight. This drives a progress wheel, and a stale wheel is worse than
/// no wheel — so it stays fast.
export const POLL_BUSY_MS = 1500;

/// While nothing is happening, which is nearly always.
///
/// The old code used `POLL_BUSY_MS` unconditionally, via a fixed `setInterval`. On Android every
/// poll is a *blocking* IPC round trip that parks the WebView's JS thread, so an open discussion
/// with an idle agent cost two of those every 1.5 s, indefinitely, on a battery device. Nothing is
/// waiting on the answer in that state: there is no wheel to keep current, only the chance that an
/// agent starts a turn from elsewhere.
export const POLL_IDLE_MS = 4000;

/// The beat for the current state.
///
/// Recomputed *after* each poll resolves rather than fixed when the timer is created — which is
/// also why the component uses a self-rescheduling `setTimeout` and not `setInterval`. A fixed
/// interval cannot change its delay, and worse, it keeps queueing ticks behind a slow one; on a
/// platform where each tick blocks the UI thread, that is how a poll becomes a permanent freeze.
export function beatMs(busy: boolean): number {
  return busy ? POLL_BUSY_MS : POLL_IDLE_MS;
}

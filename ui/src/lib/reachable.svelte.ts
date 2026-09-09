/// **Is the app on this computer still running?**
///
/// formicaria is a local server plus a browser tab. When the server stops, the tab keeps its
/// chrome and its last-rendered data and simply stops being able to fetch anything — which looks
/// far more like a broken app than a stopped one.
///
/// That is exactly what the project's only outside bug report describes
/// ([#2](https://github.com/formicaria-org/formicaria/issues/2), macOS, v0.2.1, 2026-08-30):
///
/// > *"Reloading the page didn't help. I had to rerun the start file. In general, I can't reload
/// > the page, I need to restart it to reload it."*
///
/// Their screenshot shows the toolbar intact, one pane still holding real data, and a second pane
/// whose entire body is `TypeError: Load failed` — Safari's wording for a failed `fetch()`. The
/// app had nothing to say about it, so a stopped server presented as a wedged interface, and
/// reloading (the reasonable next move) could not work *by definition*: there was nothing left to
/// reload from. Nothing on screen said so.
///
/// **The distinction this module exists to make.** `fetch` rejects with a `TypeError` when it
/// cannot reach the server at all, and resolves with a non-ok `Response` when the server answered
/// and refused. Those are opposite situations: the first means *the app is gone, restart it*, the
/// second means *the app is here and said no*. Every command already funnels through one `http()`,
/// so the difference can be noticed once, centrally, rather than by each of the several dozen
/// callers that currently swallow their own failures.
///
/// **Deliberately not a heartbeat.** Adding a poll to detect this would be a second timer whose
/// absence is itself a failure mode. Ordinary traffic is the signal: the app already beats every
/// 15 seconds and polls for vault changes, so a stopped server is noticed within seconds without
/// anything new being asked of it.

/// Consecutive network-level failures. **Two, not one**, and the number is the whole design.
///
/// A single failed request is ordinary: a command cancelled by a navigation, a socket the browser
/// opened speculatively and used too late (this server sets a 30-second read timeout, so that
/// failure is real and reachable against a perfectly healthy app), a laptop resuming from sleep.
/// Announcing "formicaria has stopped" for one of those is a false alarm about the app being
/// *dead*, which is the most alarming thing this program can say — and a false alarm there costs
/// more than a few seconds of delay in a true one.
const NEEDED = 2;

const state = $state({ misses: 0 });

/// A request failed at the network layer — nothing answered.
export function missed(): void {
  state.misses += 1;
}

/// A request completed, whatever the server said. **Any answer clears this**, including a refusal:
/// an error status is proof the app is running, which is the only question asked here.
export function reached(): void {
  state.misses = 0;
}

/// Has the app on this computer stopped answering? Reactive.
export function unreachable(): boolean {
  return state.misses >= NEEDED;
}

/// Test-only: module state outlives a render, which is the trap this repo has hit repeatedly.
export function resetReachable(): void {
  state.misses = 0;
}

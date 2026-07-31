import { describe, expect, it } from 'vitest';
import { shareSummary, type ShareStatus } from './remote';

// `isRemote` reads `location`, which is jsdom's and not settable per-test without stubbing the
// whole object; it is three comparisons and its real risk is not logic but *reliance* — so what
// is pinned here is the part with judgement in it.
//
// `shareSummary` is where this feature is most likely to lie to someone. The failure mode is not
// a crash: it is a green-looking status on a machine whose tablet cannot connect, which sends the
// user looking for the fault anywhere except the network.
describe('shareSummary', () => {
  it('never claims success from a bound socket alone', () => {
    const listening: ShareStatus = {
      state: 'listening',
      url: 'http://kestrel.local:8765',
      devices: 1,
      seen: null,
    };
    const s = shareSummary(listening);
    // A device is paired and the listener is up — everything the *server* can know looks fine,
    // and nothing has ever actually arrived. The message must say so and must name the two
    // causes the user can do something about.
    expect(s).toContain('no device has connected yet');
    expect(s.toLowerCase()).toContain('wifi');
    expect(s.toLowerCase()).toContain('firewall');
  });

  it('distinguishes never-connected from connected-and-idle', () => {
    const base = { state: 'listening', url: 'x', devices: 1 } as const;
    const never = shareSummary({ ...base, seen: null });
    const recent = shareSummary({ ...base, seen: 5 });
    const idle = shareSummary({ ...base, seen: 3600 });

    expect(never).not.toEqual(recent);
    expect(recent).toContain('connected');
    // An hour of quiet is not a fault — a tablet asleep in a bag is the ordinary case — so it
    // reports the fact rather than an alarm.
    expect(idle).toContain('60 min ago');
  });

  it('tells a fresh setup to finish pairing rather than reporting a fault', () => {
    const s = shareSummary({ state: 'listening', url: 'x', devices: 0, seen: null });
    expect(s).toContain('pair a device');
    expect(s.toLowerCase()).not.toContain('firewall');
  });

  it('passes a failure through verbatim instead of paraphrasing it', () => {
    // The reason comes from the OS (a port in use, a permission), and rewording it would drop
    // the one detail that lets someone fix it.
    const why = 'could not listen on 0.0.0.0:8765: Address already in use';
    expect(shareSummary({ state: 'failed', why })).toBe(why);
  });

  it('says plainly that nothing is shared when it is off', () => {
    const s = shareSummary({ state: 'off', devices: 0 });
    expect(s).toContain('Only this computer');
  });
});

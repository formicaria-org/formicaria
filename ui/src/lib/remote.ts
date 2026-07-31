// **Am I the computer, or a device that paired with it?**
//
// Answered from `location.hostname`, not from the server. The page already knows how it was
// reached, so asking would be a round trip to learn something we are holding — and it would be a
// round trip on the critical path of the first paint.
//
// This is a *courtesy* signal and nothing rests on it. It decides which buttons to show; the
// server refuses the same actions regardless (`REMOTE_DENIED` in `fm-serve`). If this were the
// only check, editing one line of JavaScript would be the exploit.
export const isRemote = (): boolean => {
  const h = globalThis.location?.hostname ?? '';
  return !(h === 'localhost' || h === '127.0.0.1' || h === '::1' || h === '');
};

/// What the desktop reports about sharing. Everything here is derived from what actually
/// happened — see `share.rs` — so `state` is a fact rather than a repeat of the setting.
export type ShareStatus =
  | { state: 'off'; devices: number }
  | {
      state: 'listening';
      url: string;
      devices: number;
      seen: number | null;
      /** Present only on a TLS listener. Shown so it can be compared against what the device
       *  displays *before* the certificate is installed — the only verification available on a
       *  home network, and the reason no "click through the warning" path is offered. */
      fingerprint?: string | null;
      cert_path?: string | null;
    }
  | { state: 'failed'; why: string };

/// The sentence to show under the toggle.
///
/// **`listening` is not success.** A bound socket says nothing about whether the wifi carries
/// device-to-device traffic (guest networks and most hotel wifi do not), whether a firewall is
/// dropping the port, or whether the address we printed is even the LAN one — on a machine with a
/// VPN up, the default route is the tunnel. `seen === null` means no device has *ever* got
/// through, which is the one thing that distinguishes those from "it works".
export function shareSummary(s: ShareStatus): string {
  switch (s.state) {
    case 'off':
      return 'Not shared. Only this computer can open your notes.';
    case 'failed':
      return s.why;
    case 'listening':
      if (s.seen === null) {
        return s.devices === 0
          ? 'Listening — pair a device to finish setting this up.'
          : 'Listening, but no device has connected yet. If your tablet cannot reach it, ' +
              'your wifi may block devices from talking to each other, or a firewall may be ' +
              'blocking the port.';
      }
      return s.seen < 120
        ? 'A paired device is connected.'
        : `Listening. Last device seen ${Math.round(s.seen / 60)} min ago.`;
  }
}

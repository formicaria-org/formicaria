/** Where the Gmail set-up sends the person, in one place — Settings → Mail and Collaboration's
 *  *Mail stopped* card both link here, and the manual's headings these anchors name are
 *  `docs/src/user/mail.md`'s (2026-10-16). The manual is served by `fm-serve` at `/manual/`; Gmail is
 *  read on a computer only, so there is always one to link to. */
export const MAIL_LINKS = {
  /** Google Auth Platform → Audience: *Publish app* ends the 7-day sign-out of a project in testing. */
  AUDIENCE: 'https://console.cloud.google.com/auth/audience',
  MANUAL_SETUP: '/manual/user/mail.html#set-up-your-google-cloud-client',
  MANUAL_WEEKLY: '/manual/user/mail.html#stop-the-weekly-sign-out',
} as const;

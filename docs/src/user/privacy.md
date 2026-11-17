# Privacy

Each of formicaria's protections is a switch you can turn on and off, on the phone and on the
computer. They are kept per device: your phone and your computer each have their own.

**Where:** ≡ → **Privacy** on the phone, the ant's **Privacy** in formicaria's browser on a computer,
and **Settings → Privacy** on both.

## The switches

| Switch | What it does | On at first |
|---|---|---|
| **Search with** | The engine the web field searches: DuckDuckGo, Startpage, Brave Search, Mojeek or Kagi. All private. | DuckDuckGo |
| **Block trackers** | Requests to companies that follow you from site to site are refused. On a computer this is Firefox's strict tracking protection. | yes |
| **Block ads** | Ads are refused and the empty boxes they leave are hidden. Uses more memory. On a computer it installs uBlock Origin in formicaria's browser. | no |
| **Refuse cookie pop-ups** | Consent banners are answered "reject" for you and closed. On the phone this uses DuckDuckGo's own rules; on a computer, Firefox's cookie banner reduction. | yes |
| **Clean tracking links** | Tracking parts of an address (`utm_…`, `fbclid`, `gclid`…) are taken off before the page opens. | yes |
| **Tell sites not to sell or share** | Global Privacy Control: a signal that sites must honour in some places (California, parts of the EU). | yes |
| **Secure connections only** | Pages open over https. A site with no secure version shows a page first, with **Open it anyway, this time**. | yes |
| **Block trackers in other apps** | The whole phone: see below. | no |

**On the page in front** (the phone), the sheet says what the protections did there: how many
requests were blocked, pop-ups refused, links cleaned and pages made secure.

**A site that breaks:** **Protections off for this site** turns every protection off for that site
and its subdomains. The sites are listed in the sheet, each with **Protect again**.

**Forget web data** (🔥) forgets every web page's cookies, stored data, cache and history on this
device, after asking. Your notes, the files you kept, and formicaria's own settings stay.

On a computer, the switches reach formicaria's browser within a few seconds. Firefox's own
settings stay yours: formicaria changes one only when you change its switch here.

## Block trackers in other apps (phone)

Trackers inside your *other* apps are refused for the whole phone: formicaria runs a local VPN that
answers the phone's address lookups and refuses the addresses of known trackers. Nothing leaves the
phone. Only in the F-Droid build of formicaria.

- **One VPN at a time:** while it is on, another VPN (for work or privacy) cannot run.
- Android shows its key icon while it is on, and asks once for permission.
- An app that finds its servers by its own private lookup is not seen.

## What formicaria itself sends

Nothing about you: no telemetry, no account. These features add two kinds of contact, each only
when its switch is on: the block lists are downloaded from their publishers about once a week, and
the media tools update themselves (see [Saving media](./media.md)).

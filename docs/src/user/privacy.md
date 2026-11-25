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
| **Block ads** | Ads are refused and the empty boxes they leave are hidden. On a computer it installs uBlock Origin Lite in formicaria's browser (the full uBlock Origin used five times the memory). | yes |
| **Refuse cookie pop-ups** | Consent banners are answered "reject" for you and closed. On the phone this uses DuckDuckGo's own rules; on a computer, Firefox's cookie banner reduction. | yes |
| **Clean tracking links** | Tracking parts of an address (`utm_…`, `fbclid`, `gclid`…) are taken off before the page opens. | yes |
| **Tell sites not to sell or share** | Global Privacy Control: a signal that sites must honour in some places (California, parts of the EU). | yes |
| **Always https** | Every page is tried over https first, whatever this says. Off, a site with no secure version simply opens over plain http, marked *not secure* in the address bar, as in Firefox and DuckDuckGo's browser. On, such a site shows a page first, within a few seconds, with **Open it anyway, this time**. | no |
| **Encrypted lookups** | Which sites you open is hidden from your network: the names are looked up over an encrypted connection, by Quad9 (or Mullvad DNS). That service sees the names instead. *The network's own* turns it off. On a computer, formicaria's browser; on the phone, the apps covered by *Block trackers in other apps*. | Quad9 |
| **Block trackers in other apps** | The whole phone: see below. | no |

**On the page in front** (the phone), the sheet says what the protections did there: how many
requests were blocked, pop-ups refused, links cleaned and pages made secure.

**A site that breaks:** **Protections off for this site** turns every protection off for that site
and its subdomains. The sites are listed in the sheet, each with **Protect again**.

**Forget web data** (🔥) forgets every web page's cookies, stored data, cache and history on this
device, after asking. Your notes, the files you kept, and formicaria's own settings stay.

**On a computer, Firefox's own protection does the work**: its strict tracking protection (which
also blocks known fingerprinting scripts, gives fingerprinting scripts made-up answers, and stops
trackers that bounce you through their own sites), its cookie banner reduction, and its stripping of
tracking parts from links (with formicaria's longer list of them). **More sites** adds DuckDuckGo's
cookie pop-up rules, run by formicaria's own add-on. Extensions are added only where you ask (open
source, from addons.mozilla.org), installed while their switch is on and removed when it is off:

- **Block ads** → uBlock Origin Lite (if you installed the full uBlock Origin yourself, it stays);
- **DuckDuckGo's protections too** → DuckDuckGo Privacy Essentials, its own tracker list on top of
  Firefox's.

An extension you installed yourself is never removed.

**On the phone**, formicaria's pages are protected by the same lists people use with uBlock Origin
(EasyList, EasyPrivacy, uBlock Origin's own and Peter Lowe's), with uBlock Origin's harmless
stand-ins for refused scripts, so pages break less. Tracking links are cleaned: tracking parts taken
off, and a tracker's redirect (a link through `google.com/url`, `l.facebook.com`…) skipped straight
to where it goes.

On a computer, the switches reach formicaria's browser within a few seconds. Firefox's own
settings stay yours: formicaria changes one only when you change its switch here.

## Block trackers in other apps (phone)

Trackers inside your *other* apps are refused for the whole phone: formicaria runs a local VPN that
answers the phone's address lookups and refuses the addresses of known trackers. Nothing leaves the
phone. Only in the F-Droid build of formicaria.

Its lists are made for apps: DuckDuckGo's app tracker list, EasyPrivacy and Peter Lowe's, fetched
by the phone (none is inside the app). A tracker hidden behind another name is refused too. The
lookups it lets through go encrypted to the service chosen in **Encrypted lookups**.

- **One VPN at a time:** while it is on, another VPN (for work or privacy) cannot run.
- Android shows its key icon while it is on, and asks once for permission.
- **Android's Private DNS** set to a provider sends every app's lookups there, past the block: the
  sheet says so. Set it to Automatic or Off to use the block.
- An app that looks addresses up through its own encrypted service is not seen. **Stop apps going
  around the block** refuses those services, so the app uses the phone's lookups; an app that
  insists can stop working, and can be let through.

## What is not protected

- **Your address (IP)**: sites still see where you connect from. That is a VPN's or Tor's work.
- **Your browser's fingerprint, fully**: formicaria's browser blocks known fingerprinting scripts and
  gives the rest made-up answers to the usual questions, but does not make every browser look the
  same, as Tor Browser does. On the phone, pages that read a drawing, a 3D drawing or a sound to
  tell your phone apart get tiny made-up differences, the same on that site and different on every
  other (new each time the fire button is pressed), while *Block trackers* is on.
- **On an iPhone** formicaria opens web pages in Safari, whose own protection applies.

## What formicaria itself sends

Nothing about you: no telemetry, no account. These features add contact, each only while its
switch is on: the block lists are downloaded from their publishers (about weekly; uBlock Origin's
quick fixes daily), the names of the sites you open go to the encrypted-lookup service you chose
(Quad9 at first), and the media tools update themselves (see [Saving media](./media.md)).

## The usage diary

So that formicaria can be made faster and lighter, each device keeps a **usage diary**: what
formicaria did there, never what you did with your things. It is on at first; **Settings →
Privacy → Usage diary** turns it off, shows how much is kept, and *Delete it* removes every line.

- **What it holds**: each part's memory and processor every five minutes (formicaria itself, its
  browser, the assistant); each start and stop and how it ended (closed, crashed, killed); how long
  commands and pages took; errors; which features you use, counted by name ("opened Food",
  "saved media").
- **What it never holds**: a note's text or title, a web address, a search, a mail, a name. Errors
  have anything quoted, any address, path, email or long number taken out.
- **Where it is**: on the computer, in formicaria's settings folder (`formicaria/diary/`, a file a
  day, kept 90 days and at most 100 MB). A phone keeps its own, and every hour adds its new lines
  to your first vault (`.formicaria/usage/`), so your notes' sync brings them to your computer —
  and to wherever that vault is synced. Nothing is sent anywhere else.
- **Reading it**: `pixi run usage-report` in formicaria's source folder summarises it.

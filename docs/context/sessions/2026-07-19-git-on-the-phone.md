# 2026-07-19 — Cloning a private repo on Android (twelve builds, five wrong diagnoses)

**Outcome: it works.** A private GitHub repo clones into a vault on the owner's phone, over
HTTPS, with a token. The desktop path — probe, credential, clone — was working hours earlier;
everything below is what stood between that and the same thing on a phone.

Read this before touching TLS, credentials or the Android shell. Most of it is knowledge that
cost a whole evening and is invisible from the code.

## The one that mattered: `openssl-src` builds Android with `no-stdio`

```rust
// openssl-src/src/lib.rs
// On Android it looks like not passing no-stdio may cause a build failure
if target.contains("android") { configure.arg("no-stdio"); }
```

No stdio means **no `BIO_s_file`**. `BIO_new(BIO_s_file())` returns NULL and
`X509_load_cert_file` raises `X509_R_BIO_LIB`. So on Android, with `vendored-openssl`:

> **No file-based certificate loading can ever work.** Not `SSL_CERT_FILE`, not `SSL_CERT_DIR`,
> not `GIT_OPT_SET_SSL_CERT_LOCATIONS`. Every one of them ends in a file BIO.

The device said so precisely, once the error was finally surfaced:

```text
error:05880020:x509 certificate routines::BIO lib
[path=… size=217790 mode=600 readable=true head="-----BEGIN CERTIFICATE-----"]
```

A file that is present, correctly sized, correctly permissioned, **readable by Rust**, and
starting with a valid PEM header — that OpenSSL cannot open, in the process that just wrote it.

**The fix is `GIT_OPT_ADD_SSL_X509_CERT` (option 45, not bound by `libgit2-sys`).** It takes an
`X509 *` and calls `X509_STORE_add_cert`; parsing PEM from a byte slice uses a *memory* BIO,
which `no-stdio` leaves intact. `fm_core::git_native::add_certs_from_pem` is that path, and
`fm_app::ca_bundle` collects the device's own trust store into it.

**It must run before the first `git2` call in the process**, and it must initialise libgit2
itself: calling `git_libgit2_opts` raw skips the `init()` every `git2::opts::*` wrapper does, and
`git_openssl__add_x509_cert` then dereferences a NULL `git__ssl_ctx` — a launch crash, shipped
once to a real phone.

## Four other findings worth keeping

1. **Android names CA files with the pre-1.0.0 subject hash.** `01419da9.0` on disk;
   `openssl -subject_hash` says `8d89cda1`; `-subject_hash_old` says `01419da9`. So
   `SSL_CERT_DIR` pointed at Android's store finds *nothing* and fails silently — a fix that
   looks right and changes nothing. (Moot now, but it is the trap that eats the first day.)
2. **The Conscrypt APEX store supersedes `/system`; it does not extend it.** 145 vs 149 entries
   on the device — those four are roots the platform **dropped**. Unioning them re-trusts what
   Android stopped trusting. Priority list, first non-empty wins.
3. **`Repository::clone` attaches no callbacks.** It builds its own default fetch options, so a
   private remote never sees the token. It was the only network call here without credentials,
   and a `file://` differential test cannot catch it because a local path never authenticates.
4. **libgit2's "remote authentication required but no callback set" is misleading.** It is what
   you get when the callback *runs* and returns a credential the transport cannot use — our
   `Cred::default()` (NTLM/Negotiate) against a forge wanting basic auth. Measured in isolation:
   callback invoked, handed `USER_PASS_PLAINTEXT`, returns default, error says "no callback set".
   **This survived the correct fix and made it look broken.** The fallback now refuses a
   username/password ask in plain words.

## Process lessons, which cost more than the bugs

- **Rust's stderr goes nowhere on Android, and MIUI suppresses app logcat besides.** Every
  `eprintln!` diagnostic wrote into a void for several builds. `android_logger` fixes the first
  half; on a Xiaomi device the only reliable channel is **the app's own UI** (hence `config`
  reporting `ca_bundle`). Verified working on the emulator, silent on the phone.
- **The emulator existed in `pixi.toml` the whole time and went unused.** It catches launch
  crashes in seconds and has working logcat. Using it would have prevented shipping a SIGSEGV to
  a real phone. It is now the gate before any device install.
- **`str.replace` in a script fails silently.** One commit's message described work that had not
  happened, because the replacement never matched. Grep the *shipped* `.so` for the string a
  change was supposed to add — it takes seconds and caught exactly this.
- **`pgrep -f <name>` matches its own command line.** Recorded already for `pkill -f fm-serve`;
  walked into twice more while checking whether an emulator was running. `ps -eo pid,comm`.
- **Verify the artifact against the commit, not the build's exit code.** An APK finished 8
  seconds *before* a fix was committed and would have been installed as current.

## What is still not done

- **The Android token is app-private storage, not the Keystore** the owner chose. Kernel-isolated
  from other apps — genuinely better than the plaintext `~/.git-credentials` a Linux desktop
  leaves — but not hardware-backed. Needs a Tauri Android plugin layer this repo does not have.
- **The phone toolbar scrolls Settings off-screen** (`overflow-x: auto`), which made the only
  diagnostic channel unreachable exactly when it was needed. On a phone these belong in the
  bottom bar.
- **"Save token" is a separate button from "Join"**, and typing a token then pressing Join
  silently does nothing with it.

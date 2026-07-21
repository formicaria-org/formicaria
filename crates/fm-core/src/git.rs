//! Version the vault with git (subprocess) — the same "shell out to the tool
//! that already solves this" stance as [`crate::backup`]. Git is the durable
//! safety net: the notes are plain text, so a full history of every edit lives in
//! an ordinary repo the user can push to GitHub. Heavy blobs and the disposable
//! index are kept out (they sync out-of-band), so the notes repo stays small and
//! clonable forever.
//!
//! The vault is its **own** repo, independent of the application's source tree —
//! detected by a real `.git` under the vault, never by `git rev-parse` (which
//! would walk up and find a parent repo when the vault sits inside one).

use crate::StoreError;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The one remote we manage. Git's own default name, so a vault stays an
/// ordinary repo that behaves as expected in a terminal.
pub(crate) const REMOTE: &str = "origin";

/// The stand-in committer written for a vault nobody else can see. Git refuses to
/// commit without *some* identity and cannot invent one on a machine whose hostname
/// is not a FQDN — which is most of them — so without this a researcher who never
/// configured git could not save a note at all.
///
/// It is deliberately **not a person**: [`identity`] reports it as absent and
/// [`set_remote`] refuses to give a vault an audience while it stands. That is what
/// turns "no git config" into a single question, asked once, at the only moment the
/// answer matters — and it lets a vault that has been running on the placeholder for
/// months heal itself the moment the user answers.
///
/// # This is a sentinel matched by value — changing it has consequences
///
/// [`identity`] compares `user.email` against this exact string to decide "nobody real
/// signs this vault". A vault's `.git/config` is per-machine, so we cannot migrate the
/// ones we cannot see: if this literal ever changes again, every vault still carrying the
/// old value silently acquires a *real* identity, [`set_remote`] stops asking who they
/// are, and the provenance hole this exists to close is quietly open. It was safe to
/// change once, while the only vaults in the world were the author's and none were on the
/// placeholder. That will not be true a second time.
pub(crate) const PLACEHOLDER_NAME: &str = "formicaria";
pub(crate) const PLACEHOLDER_EMAIL: &str = "formicaria@localhost";


// ---------------------------------------------------------------------------------------
// Rules shared with the `native-git` backend (`crate::git_native`).
//
// These are here, once, because a second implementation of the sync path that disagrees with
// this one about *what is allowed* is the corruption this whole design exists to prevent. A
// backend may differ in how it talks to a repository; it may not differ in whether an email
// without an `@` is acceptable, or whether a vault may gain a remote while nobody real signs
// it. Two copies of a rule are two rules.
// ---------------------------------------------------------------------------------------

/// The identity rules. Trimmed values back, or the reason they are refused.
pub(crate) fn check_identity(name: &str, email: &str) -> Result<(String, String), StoreError> {
    let (name, email) = (name.trim(), email.trim());
    if name.is_empty() || email.is_empty() {
        return Err(StoreError::Io("a git identity needs both a name and an email".into()));
    }
    // Not validation — git does none either, and an address this app rejects is an address the
    // user cannot use. This catches only the mistake that is invisible afterwards: a name typed
    // into the email box, signed into history forever.
    if !email.contains('@') || email == PLACEHOLDER_EMAIL {
        return Err(StoreError::Io(format!("'{email}' is not an email address")));
    }
    Ok((name.to_string(), email.to_string()))
}

/// Turn a raw `user.name`/`user.email` pair into an [`Identity`], or `None` when nobody real
/// signs this vault. The placeholder is a sentinel, not a person.
pub(crate) fn identity_from(name: String, email: String) -> Option<Identity> {
    (email != PLACEHOLDER_EMAIL).then_some(Identity { name, email })
}

/// Whether this vault may be given a remote yet — the moment it stops being private.
pub(crate) fn check_remote_allowed(
    _vault: &Path,
    url: &str,
    has_identity: bool,
) -> Result<(), StoreError> {
    // `git remote add origin ""` *succeeds*, and the resulting remote then reports its own name
    // as its URL, so the vault would claim a destination it does not have.
    if url.trim().is_empty() {
        return Err(StoreError::Io("a remote needs a URL".into()));
    }
    // From here on every commit carries a name into somebody else's clone, and git history is
    // forever. If that name is the placeholder, every "who touched this?" the product can ever
    // answer is the same fake.
    if !has_identity {
        return Err(StoreError::Io(
            "tell us who you are first — your name and email sign every commit you share".into(),
        ));
    }
    Ok(())
}

/// Whether this destination can be cloned into. Refuses *before* anything is written, so a
/// failure never leaves a half-made vault in a directory too non-empty to retry into.
pub(crate) fn check_clone_dest(url: &str, dest: &Path) -> Result<(), StoreError> {
    if url.trim().is_empty() {
        return Err(StoreError::Io("a clone needs a remote URL".into()));
    }
    if dest.exists() && dest.read_dir().is_ok_and(|mut d| d.next().is_some()) {
        return Err(StoreError::Io(format!(
            "{} already exists and is not empty — clone into a new directory",
            dest.display()
        )));
    }
    Ok(())
}

/// The files that make a repo a *vault*: the ignore rules and the merge attribute. Append-never-
/// skip, because a repo you already own has both files and "skip if present" meant neither rule
/// ever landed.
pub(crate) fn write_vault_files(vault: &Path) -> Result<(), StoreError> {
    write_gitignore(vault)?;
    write_gitattributes(vault)?;
    // Best-effort on a native backend: there is no `fm` binary beside a phone app to point a
    // driver at, and libgit2 could not invoke it anyway. Failing here would refuse to open a
    // perfectly good vault over a driver that platform can never use.
    let _ = install_merge_driver(vault);
    Ok(())
}

/// Who a vault's commits are attributed to — the name a collaborator sees when they
/// ask who touched a note.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

/// Is git on this machine at all?
///
/// **Git is optional, and this is the function that says so out loud.** A vault is a
/// directory of Markdown files; everything that makes formicaria a notebook — capture,
/// edit, board, agenda, search — is `FileStore` over those files and never spawns git.
/// Someone with no git has a complete, working, single-PC notebook. What they do not have
/// is **history**: the local undo that reaches past this session, and the backup and
/// collaboration built on top of it.
///
/// So git is a **capability to declare, not a dependency to assume**. Without this the
/// debounced auto-commit spawns git every five seconds forever, fails, and is swallowed —
/// leaving a vault quietly unversioned, which you discover on the day you need the history
/// and it is not there.
///
/// Cached: git does not appear halfway through a run, and this is asked on the heartbeat.
pub fn available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// A git invocation with the non-interactive guards, but no working directory yet.
///
/// Split out for [`clone`], which is the one operation that cannot use [`git`]: the
/// directory it would `-C` into does not exist until the clone creates it. The guards are
/// the load-bearing half and must not be duplicated by hand — a clone that prompts is a
/// clone that hangs a thread-per-connection server forever.
fn git_cmd() -> Command {
    let mut c = Command::new("git");
    // Never let git stop to ask a human. fm-serve is thread-per-connection and
    // has no TTY, so a credential or host-key prompt would hang the request
    // forever rather than fail. Authentication is whatever the user's ssh-agent
    // or credential helper already provides — this app holds no secret of its own.
    c.env("GIT_TERMINAL_PROMPT", "0");
    c.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    c
}

fn git(vault: &Path) -> Command {
    let mut c = git_cmd();
    c.arg("-C").arg(vault);
    c
}

/// What asking a remote, without cloning it, told us.
///
/// The point is to separate the three failures a user cannot tell apart from a clone's output:
/// a typo'd URL, a repo that needs credentials this machine does not have, and being offline.
/// They need completely different next steps, and "could not read Username for 'https://…'"
/// says none of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// It answered, and we may read it — either it is public or our credentials already work.
    Reachable,
    /// It exists (or at least the host does) but will not talk to us unauthenticated.
    NeedsAuth,
    /// No such repo, host not found, offline. Carries what the tool actually said.
    Unreachable(String),
}

/// Ask a remote whether we could clone it — **without cloning it**.
///
/// `ls-remote` fetches refs only and downloads no objects, so this is cheap enough to run
/// while the user is still typing a URL into a form. `GIT_TERMINAL_PROMPT=0` is what turns a
/// credential prompt into an *answer* rather than a hang: with no TTY a prompt would block a
/// thread-per-connection server forever, and on a phone there is no terminal to prompt on at
/// all.
pub fn probe(url: &str) -> Probe {
    if !available() {
        return Probe::Unreachable("git is not installed on this machine".into());
    }
    let out = match git_cmd().arg("ls-remote").arg(url.trim()).output() {
        Ok(o) => o,
        Err(e) => return Probe::Unreachable(format!("could not run git: {e}")),
    };
    if out.status.success() {
        return Probe::Reachable;
    }
    Probe::from_stderr(&String::from_utf8_lossy(&out.stderr))
}

impl Probe {
    /// Classify a failure by what the tool said. **Matched on substrings deliberately**: the
    /// exit code is 128 for every one of these, so the message is the only signal there is.
    /// Anything unrecognised stays `Unreachable` with the original text — a wrong *guess* here
    /// would send someone to configure credentials for a URL they simply mistyped.
    pub fn from_stderr(stderr: &str) -> Probe {
        let s = stderr.to_lowercase();
        let auth = [
            "authentication failed",
            "could not read username",
            "could not read password",
            "terminal prompts disabled",
            "permission denied",
            "invalid username or password",
            "authentication required",
            "403 forbidden",
            // **Deliberately NOT "please make sure you have the correct access rights".** Git
            // prints that for a repository that simply does not exist — the full sentence is
            // "...correct access rights and the repository exists", which names both causes and
            // therefore distinguishes neither. Matching it sent a plain typo to the credentials
            // advice, which is the one wrong answer this classifier must never give.
        ];
        if auth.iter().any(|m| s.contains(m)) {
            return Probe::NeedsAuth;
        }
        Probe::Unreachable(stderr.trim().to_string())
    }
}

/// How this machine is set up to authenticate to git, in the terms a user can act on.
///
/// **Names the helper, never a secret.** `credential.helper` is a program name, and the whole
/// value of reporting it is that the answer is often "you have one, and it is the bad one":
/// `store` keeps credentials as **plaintext** in `~/.git-credentials`, which most people who
/// have it did not choose on purpose — `git` writes it when a tutorial says to.
///
/// `None` means nothing is configured, which is not a problem by itself: SSH remotes
/// authenticate through the agent and never consult a helper at all.
pub fn credential_helper() -> Option<String> {
    if !available() {
        return None;
    }
    let out = git_cmd().args(["config", "--get", "credential.helper"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// Hand a credential to **git's own credential helper**, so the whole machine gets it.
///
/// # Why this, rather than a token formicaria keeps
///
/// `git credential approve` is the documented way to *write* into whatever helper is
/// configured — libsecret or the GNOME keyring on Linux, the macOS Keychain, the Windows
/// Credential Manager. So the user pastes a token into formicaria once and it lands in the
/// platform's real credential store, where `git` in their terminal and every other tool finds
/// it too.
///
/// That keeps **"the app holds no secret of its own"** literally true on the desktop rather
/// than approximately true: nothing is written by us, nothing is ours to leak, and there is no
/// second copy to go stale when they rotate the token. It is the same stance as shelling out
/// to restic instead of reimplementing dedup — the tool already solves this whole.
///
/// **Silently a no-op when no helper is configured**, which is git's behaviour and not
/// something we can change: with nowhere to store it, `approve` accepts the input and drops it.
/// [`helper_advice`] exists so a caller can tell the user that *before* they paste anything.
///
/// The secret goes in on **stdin**, never as an argument — a command line is visible to every
/// process on the machine via `/proc`, and that is the one mistake this function must not make.
pub fn credential_approve(url: &str, username: &str, secret: &str) -> Result<(), StoreError> {
    use std::io::Write;
    if !available() {
        return Err(StoreError::Io("git is not installed on this machine".into()));
    }
    if secret.trim().is_empty() {
        return Err(StoreError::Io("a credential needs a token or password".into()));
    }
    // Newlines would forge extra fields in git's key=value protocol.
    if [url, username, secret].iter().any(|v| v.contains('\n') || v.contains('\r')) {
        return Err(StoreError::Io("a credential may not contain a line break".into()));
    }
    let mut child = git_cmd()
        .args(["credential", "approve"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(spawn)?;
    {
        let mut si = child.stdin.take().ok_or_else(|| StoreError::Io("no stdin".into()))?;
        // `url=` is git's own shorthand: it parses protocol, host and path out for us, so we
        // never have to reimplement URL splitting and get it subtly different from git.
        let payload = format!("url={}\nusername={}\npassword={}\n\n", url.trim(), username, secret);
        si.write_all(payload.as_bytes()).map_err(io)?;
    }
    let out = child.wait_with_output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git credential approve", &out));
    }
    Ok(())
}

/// Does a usable credential for this URL already exist on this machine?
///
/// The answer to *"they could have been already set, like in this repo"* — and the reason the
/// form should not ask for a token it does not need. `git credential fill` consults the
/// configured helper; `GIT_TERMINAL_PROMPT=0` (set by [`git_cmd`]) turns "nothing stored" into
/// a failure instead of a prompt that would hang a server with no TTY.
///
/// **The credential itself is never returned, logged, or looked at** — only whether one came
/// back. There is no caller that needs the value, so there is no signature here that could
/// leak it.
pub fn credential_exists(url: &str) -> bool {
    use std::io::Write;
    if !available() || url.trim().is_empty() {
        return false;
    }
    let Ok(mut child) = git_cmd()
        .args(["credential", "fill"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(format!("url={}\n\n", url.trim()).as_bytes());
    }
    let Ok(out) = child.wait_with_output() else { return false };
    if !out.status.success() {
        return false;
    }
    // A helper that answered gives back a non-empty `password=` line. Checked as a prefix on a
    // line rather than a substring, so a *host* containing the word never reads as a hit.
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|l| l.strip_prefix("password=").is_some_and(|v| !v.is_empty()))
}

/// What this machine is set up to remember credentials with, and what it should be.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HelperAdvice {
    /// The configured helper's name, or `None`. A program name — never a secret.
    pub configured: Option<String>,
    /// It keeps credentials in **plaintext**. Only `store` does this, and most people running
    /// it were told to by a tutorial rather than choosing it.
    pub plaintext: bool,
    /// A helper that exists on this machine and would encrypt at rest, when the configured one
    /// does not. `None` when the current setup is already fine, or when we found nothing better
    /// to suggest — an honest absence beats naming a program that is not installed.
    pub better: Option<String>,
}

/// Which credential helper this machine uses, and whether there is a better one available.
///
/// **Suggests only what is actually installed.** Telling someone to configure
/// `credential.helper libsecret` on a box where it was never built is how advice becomes noise;
/// the candidates are probed by asking git whether it can run them.
pub fn helper_advice() -> HelperAdvice {
    let configured = credential_helper();
    let plaintext = configured.as_deref() == Some("store");
    let needs_better = configured.is_none() || plaintext;
    let better = needs_better.then(pick_helper).flatten();
    HelperAdvice { configured, plaintext, better }
}

/// The best credential helper installed here, or `None`.
///
/// Ordered by what each actually protects: a platform keychain encrypts at rest and unlocks
/// with the login session; `cache` only holds things in memory for a while, which is still
/// strictly better than a plaintext file on disk. `store` is never suggested — it is the thing
/// we are suggesting a way out of.
fn pick_helper() -> Option<String> {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &["osxkeychain", "cache"]
    } else if cfg!(target_os = "windows") {
        &["manager", "wincred", "cache"]
    } else {
        &["libsecret", "gnome-keyring", "cache"]
    };
    candidates.iter().find(|h| helper_runs(h)).map(|h| (*h).to_string())
}

/// Can git actually run this helper? `git credential-<name>` exits non-zero on a bad argument
/// but reports "not found" differently, which is the distinction we need.
fn helper_runs(name: &str) -> bool {
    Command::new(format!("git-credential-{name}"))
        .arg("--help")
        .output()
        .map(|o| o.status.success() || !o.stderr.is_empty())
        .unwrap_or(false)
}

/// Clone `url` into `dest` and make the result a formicaria vault.
///
/// **New code, on every possible backend.** `git.rs` never had a clone — every plan document
/// that described the mobile M1 as "porting" this was wrong about what exists, and the
/// correction is recorded in `docs/context/mobile-design.md`. It is written here, on the
/// subprocess backend, because that is where it is testable today and because it is a
/// prerequisite under every resolution of the git-backend question.
///
/// Calls [`ensure_repo`] itself rather than leaving it to the caller. A fresh clone needs the
/// `.gitattributes` merge attribute *and* the `merge.fm.driver` definition — and the driver
/// definition deliberately does not travel in a repo, so a collaborator who clones and does
/// not run this silently falls back to git's plain text merge and hits the `updated:` conflict
/// on every concurrent edit. That is the exact trap `a_fresh_clone_gets_both_halves_of_the_driver`
/// exists to catch, and making it the caller's job is how it would be forgotten.
///
/// Does **not** set an identity: that is [`set_identity`], and the caller must do it before the
/// first commit or every commit from this machine is attributed to the placeholder. A clone has
/// an audience by definition, which is precisely when that matters.
pub fn clone(url: &str, dest: &Path) -> Result<(), StoreError> {
    if !available() {
        return Err(StoreError::Io(
            "git is not available on this machine, so there is nothing to clone with".into(),
        ));
    }
    // Refuse before writing: `git clone` says it in terms of the directory rather than of what
    // the user was doing, and by then it has already created it.
    check_clone_dest(url, dest)?;

    let out = git_cmd().arg("clone").arg(url.trim()).arg(dest).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git clone", &out));
    }

    // The clone is a repo, but not yet a *vault*: the ignore rules and the merge driver are
    // what make it one, and `ensure_repo` appends whatever the remote did not already carry.
    ensure_repo(dest)?;
    Ok(())
}

/// Initialize the vault as its own git repo if it isn't one yet, giving it a
/// committer identity and a `.gitignore` that keeps the per-machine index,
/// regenerable thumbnails, and heavy blobs out of history. Returns true if the
/// repo was created.
pub fn ensure_repo(vault: &Path) -> Result<bool, StoreError> {
    // A repo root always has a `.git` entry (dir or, for worktrees, a file). This
    // is the *only* correct probe here: `git rev-parse` walks upward and would
    // report the parent repo when the vault is nested inside one.
    if vault.join(".git").exists() {
        // A repo we did not create — `git init`ed by hand, **cloned from a collaborator**,
        // or a project repo being adopted as a vault — still needs the ignore rules and the
        // merge attribute. Both writers **append what is missing** rather than skipping a
        // file that exists, which is the only version that works for a repo you already own:
        // it already has both files, so "skip if present" meant neither rule ever landed.
        write_gitignore(vault)?;
        write_gitattributes(vault)?;
        install_merge_driver(vault)?;
        return Ok(false);
    }
    std::fs::create_dir_all(vault).map_err(io)?;
    let out = git(vault).arg("init").output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git init", &out));
    }
    ensure_identity(vault);
    write_gitignore(vault)?;
    write_gitattributes(vault)?;
    install_merge_driver(vault)?;
    Ok(true)
}

/// Ask git to merge notes through us, and to leave their bytes alone. Tracked, so it
/// travels to every clone — which is exactly half the job, and the half that is *not*
/// enough (see [`install_merge_driver`]). Idempotent like `.gitignore`: the rule is
/// **appended when missing**, and a file that already has it is not rewritten.
///
/// `eol=lf` is not tidiness. Git's default on Windows rewrites text to CRLF on checkout,
/// so the same note would be different bytes on different machines — and byte-for-byte
/// round-tripping is the invariant files-as-truth rests on. It would also hand the `.md`
/// merge driver two files that differ on every single line, turning every pull between a
/// Windows and a Linux collaborator into a whole-file conflict. (`from_file` tolerates
/// CRLF anyway, because an editor can still produce it — but a vault should not.)
fn write_gitattributes(vault: &Path) -> Result<(), StoreError> {
    // **Append, never skip.** This used to return early when the file existed, which read
    // as politeness and behaved as sabotage: *every* real repo already has a
    // `.gitattributes`, so the moment a vault is a project you already own — which is the
    // whole point of Track V — `merge=fm` never lands. Git then falls back to its built-in
    // text merge, every concurrent edit collides on the `updated:` line we rewrite on each
    // save, and the markers land inside the YAML fence where `from_file` rejects them. That
    // is exactly the disaster Track C Phase 1 exists to prevent, reintroduced by conversion,
    // and it is silent: nothing anywhere says the driver was meant to be running.
    ensure_line(&vault.join(".gitattributes"), "*.md merge=fm text eol=lf")?;
    // The manifest is a `sha256 -> size` map of content-addressed blobs, and it is *staged on
    // every commit*. Merged as ordinary text, two people ingesting a file on the same day get
    // `<<<<<<<` markers inside a JSON document — a file no user wrote, can read, or can resolve
    // — and once it is conflicted `commit_all` correctly refuses to commit anything else in the
    // vault, so the whole thing silently stops recording. Merged as a union it cannot conflict
    // at all: the key *is* the content, so agreement is structural.
    ensure_line(&vault.join(".gitattributes"), "manifest.json merge=fm-manifest text eol=lf")
}

/// Make sure `line` is present in a line-oriented config file, leaving every other byte of
/// it alone. Creates the file when absent.
///
/// The file belongs to the user — they may have rules of their own in it, and a writer that
/// rewrites what it did not author is a writer you cannot point at someone's repo. So this
/// only ever adds, and only what is missing.
fn ensure_line(path: &Path, line: &str) -> Result<(), StoreError> {
    let existing = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(io(e)),
    };
    if existing.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    let mut out = existing;
    // Don't glue our line onto the end of theirs if the file lacked a trailing newline.
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(line);
    out.push('\n');
    std::fs::write(path, out).map_err(io)
}

/// Teach *this clone* what `merge=fm` actually runs.
///
/// The trap: `.gitattributes` is tracked and travels, but the `merge.fm.driver`
/// **definition** lives in `.git/config`, which deliberately does not — git will not
/// let a repo ship a command line that then executes on your machine. So a
/// collaborator who clones and never runs this app silently falls back to git's
/// built-in text merge and gets the `updated:` conflict on every concurrent edit,
/// with no sign that anything was meant to prevent it. Install it on every
/// `ensure_repo`, exactly as we already write `.gitignore` and the identity.
///
/// Install nothing unless we can name a binary that **exists**, because a driver that
/// fails to run is far worse than no driver at all: git takes any non-zero exit as
/// "conflict", and the `%A` it hands back is untouched — i.e. *our* version, with no
/// markers in it. The user sees a conflict, opens a file that looks completely normal,
/// resolves it, and has silently deleted their collaborator's edit. An undefined
/// driver, by contrast, degrades to git's built-in text merge: uglier, and correct.
fn install_merge_driver(vault: &Path) -> Result<(), StoreError> {
    // **Nothing to point at means actively removing what is there**, not leaving it. This
    // used to `return Ok(())`, which reads as "install nothing" and behaves as "keep whatever
    // the last install wrote" — and what it wrote is an *absolute* path (see
    // [`merge_command`]). Every way that path goes stale is the silent-data-loss case above,
    // not a hypothetical: reinstalling to a different prefix, a dev build where a release one
    // ran, a package shipping `fm-serve` without `fm`, a vault directory copied between
    // machines (`.git/config` travels with a copy even though it does not travel with a
    // clone), or mobile, which has no `fm` beside it at all.
    //
    // Removing it degrades to git's built-in text merge: uglier, and *visible*. Leaving a dead
    // path degrades to a conflict on a file that looks clean. Only one of those is survivable.
    let Some(exe) = merge_command() else { return clear_merge_driver(vault) };
    for (key, value) in [
        ("merge.fm.name", "formicaria frontmatter-aware note merge".to_string()),
        // %O base, %A ours (and where the answer goes), %B theirs, %L marker size.
        ("merge.fm.driver", format!("'{exe}' merge-md %O %A %B %L")),
        ("merge.fm-manifest.name", "formicaria blob-inventory union merge".to_string()),
        // No %L: a union has no conflict to mark.
        ("merge.fm-manifest.driver", format!("'{exe}' merge-manifest %O %A %B")),
    ] {
        let out = git(vault).args(["config", &key, &value]).output().map_err(spawn)?;
        if !out.status.success() {
            return Err(failed("git config", &out));
        }
    }
    Ok(())
}

/// Forget a `merge.fm` definition this machine cannot honour.
///
/// Best-effort by construction: `git config --unset` exits 5 when the key is simply not
/// there, which is the ordinary case on every vault that never had a driver, and is not a
/// failure of anything. The only outcome that matters is that no *stale* definition survives
/// this call, and an unset that could not run leaves us no worse than before.
fn clear_merge_driver(vault: &Path) -> Result<(), StoreError> {
    for key in
        ["merge.fm.driver", "merge.fm.name", "merge.fm-manifest.driver", "merge.fm-manifest.name"]
    {
        let _ = git(vault).args(["config", "--unset-all", key]).output();
    }
    Ok(())
}

/// An absolute path to the `fm` binary, or `None` if we cannot find one.
///
/// The one beside whatever is running now: the release bundle ships `fm` and
/// `fm-serve` side by side, so this resolves without asking the user to put anything
/// on PATH — they launch from a desktop icon and have no PATH we chose. **Never a bare
/// `fm` hoping PATH will answer**: PATH at `git pull` time is not PATH now, and being
/// wrong about that is the silent-data-loss case above. If it isn't there, say so and
/// let git merge the way it always has.
fn merge_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let fm = exe.parent()?.join(if cfg!(windows) { "fm.exe" } else { "fm" });
    fm.exists().then(|| fm.to_string_lossy().into_owned())
}

/// Give a brand-new vault the [placeholder](PLACEHOLDER_EMAIL) committer when the
/// user has no git config of their own, so the first commit never fails on a fresh
/// machine. Repo-local, so it never touches the user's global settings, and
/// best-effort: a vault that cannot be configured will surface that at commit time
/// with git's own message, which is better than ours.
fn ensure_identity(vault: &Path) {
    let missing = |k: &str| config(vault, k).is_none();
    if missing("user.email") {
        let _ = git(vault).args(["config", "user.email", PLACEHOLDER_EMAIL]).output();
    }
    if missing("user.name") {
        let _ = git(vault).args(["config", "user.name", PLACEHOLDER_NAME]).output();
    }
}

/// One effective git config value — local, global or system, resolved exactly as
/// git itself would for a commit made in this vault. `None` for unset *and* empty:
/// a key set to nothing is not an answer.
fn config(vault: &Path, key: &str) -> Option<String> {
    let out = git(vault).args(["config", key]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// One git config value read from **this repo's own config only** — never falling back to
/// the user's global or the system file.
///
/// The distinction matters exactly once, in [`forget_identity`]: "does this directory carry
/// somebody else's identity?" and "does this user have an identity?" are different questions,
/// and [`config`] answers the second. Asking the wrong one would report every user on earth
/// as carrying an inherited identity.
fn config_local(vault: &Path, key: &str) -> Option<String> {
    let out = git(vault).args(["config", "--local", key]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// Forget a committer identity that came in with a copied vault. Returns whether there was
/// one to forget.
///
/// **For freshly-acquired directories only** — see [`crate::acquire::naturalise`], the sole
/// caller. Clearing the identity on a vault someone already works in would silently detach
/// their name from their own commits.
///
/// A `git clone` never needs this (git declines to carry `.git/config`, which is the whole
/// reason [`install_merge_driver`] has to exist). A **verbatim directory copy does**, and it
/// is the one transport shape that arrives with the sender's name already configured — after
/// which every commit this machine makes is attributed to them, in a shared history, with
/// nothing on screen to suggest it.
///
/// Best-effort: `--unset-all` exits non-zero when the key was never set, which is the
/// ordinary case and not a failure of anything.
pub fn forget_identity(vault: &Path) -> bool {
    let had =
        config_local(vault, "user.name").is_some() || config_local(vault, "user.email").is_some();
    for key in ["user.name", "user.email"] {
        let _ = git(vault).args(["config", "--local", "--unset-all", key]).output();
    }
    had
}

/// Who this vault's commits are attributed to, or `None` when nobody real is —
/// either git has no identity at all, or it still holds our placeholder. Reporting
/// our own stand-in as absent is deliberate: it is the difference between a vault
/// that asks once and a vault that quietly signs a shared history `formicaria`.
pub fn identity(vault: &Path) -> Option<Identity> {
    if !vault.join(".git").exists() {
        return None;
    }
    let email = config(vault, "user.email")?;
    identity_from(config(vault, "user.name")?, email)
}

/// Record who the user is, repo-locally — the answer to the question
/// [`set_remote`] asks. Scoped to this vault because a vault is an audience: the
/// name you push to a lab repo need not be the one on your personal notes, and this
/// app has no business editing anyone's global git config.
pub fn set_identity(vault: &Path, name: &str, email: &str) -> Result<(), StoreError> {
    let (name, email) = check_identity(name, email)?;
    let (name, email) = (name.as_str(), email.as_str());
    ensure_repo(vault)?;
    for (key, value) in [("user.name", name), ("user.email", email)] {
        let out = git(vault).args(["config", key, value]).output().map_err(spawn)?;
        if !out.status.success() {
            return Err(failed("git config", &out));
        }
    }
    Ok(())
}

fn write_gitignore(vault: &Path) -> Result<(), StoreError> {
    // index.sqlite is per-machine and must never travel (the DB-corruption-by-sync
    // lesson); derived/ thumbnails regenerate; blobs sync out-of-band, never here.
    //
    // **Append, never skip** — same reason as `.gitattributes`, with a louder failure. A
    // repo that already has a `.gitignore` (i.e. every real one) used to get none of these
    // lines, so the debounced auto-commit swept `blobs/` and `index.sqlite` into history and
    // the next push shipped every PDF and video to the remote — plus a per-machine SQLite
    // file that must never travel.
    let path = vault.join(".gitignore");
    for line in ["index.sqlite", "derived/", "blobs/"] {
        ensure_line(&path, line)?;
    }
    Ok(())
}

/// Blob files this vault is willing to put in git: every one at or under `git_assets_max`.
///
/// **Empty unless the vault opts in**, so the walk costs nothing in the default configuration —
/// which matters, because `commit_all` runs on a 5-second debounce.
///
/// Already-tracked blobs above a *newly lowered* threshold are deliberately left alone. Untracking
/// them would not remove them from history — the bytes are in every clone the moment they are
/// pushed — so it would cost a confusing deletion commit and buy nothing. Lowering the limit
/// governs what travels *next*, which is the only thing it can honestly govern.
fn blobs_within(vault: &Path) -> Result<Vec<String>, StoreError> {
    let Some(max) = crate::descriptor::Descriptor::read(vault)?.git_assets_max else {
        return Ok(Vec::new());
    };
    let store = crate::blob::BlobStore::new(vault);
    let mut out: Vec<String> = store
        .blob_paths()
        .into_iter()
        .filter(|p| std::fs::metadata(p).map(|m| m.len() <= max && m.len() > 0).unwrap_or(false))
        .filter_map(|p| relative(vault, &p))
        .collect();
    out.sort();
    Ok(out)
}

/// Stage the vault's own files and commit them. Returns false — not an error — when there
/// is nothing of ours to commit; that is the common case for a debounced auto-commit and
/// must not surface as a failure.
///
/// **Scoped on purpose.** A vault may be a repo that also holds code or a manuscript, and
/// this runs every few seconds. It must never touch a file formicaria did not write, and
/// must never disturb an index the user staged themselves.
pub fn commit_all(vault: &Path, message: &str, paths: &[PathBuf]) -> Result<bool, StoreError> {
    ensure_repo(vault)?;
    let status = git(vault).arg("status").arg("--porcelain").output().map_err(spawn)?;
    if !status.status.success() {
        return Err(failed("git status", &status));
    }
    if status.stdout.is_empty() {
        return Ok(false);
    }
    // Never commit during a conflicted merge. `add -A` would stage the conflict
    // markers, which git reads as "the human resolved it", and the commit would
    // enshrine `<<<<<<<` as the note's content and push it. The auto-commit is
    // debounced 5s after any write, so a pull that conflicts hits this within
    // seconds — it is the default path, not an edge case. Not an error: "nothing
    // committed" is exactly what the caller already handles.
    if String::from_utf8_lossy(&status.stdout).lines().any(unmerged) {
        return Ok(false);
    }
    // **Stage exactly the files we wrote, never `-A` and never a directory.**
    //
    // A vault is increasingly *a repo you already have* — notes beside the code or
    // manuscript they describe. `git add -A` there is not a tidy default, it is a
    // second author: every 5 seconds it stages your half-written function, your
    // mid-sentence paragraph, and whatever you had carefully staged for a commit of your
    // own, then commits the lot under `auto:`. Losing a curated index that way is not
    // recoverable by re-running anything.
    //
    // `paths` is what `FileStore::put`/`delete` recorded — the only thing that actually
    // knows which files are ours. Staging a *directory* was the previous approximation, and
    // it still caught a note you were hand-editing in Vim, because in a project vault the
    // notes directory may well be `docs/`.
    //
    // The config files we author are included because we author them: `ensure_repo` writes
    // the ignore rules and the merge attribute, and they must travel.
    let mut owned: Vec<String> =
        paths.iter().filter_map(|p| relative(vault, p)).collect();
    for f in [".gitattributes", ".gitignore", "manifest.json"] {
        if vault.join(f).exists() {
            owned.push(f.to_string());
        }
    }
    // **Drop phantom paths git cannot name.** A note captured *and* deleted before its first
    // commit is in `paths` (`FileStore` recorded the write and the delete) yet is neither on disk
    // nor tracked. `git add -A -- <it>` rejects the WHOLE batch on it ("pathspec did not match any
    // file"), which pauses history and backup for every *other* change too — a create-then-delete
    // must not brick the vault. Such a path has nothing to record (git never saw it), so keep only
    // the nameable ones: a path that is tracked (a real modify/delete) or exists on disk (an add).
    if !owned.is_empty() {
        let ls = git(vault).arg("ls-files").arg("-z").arg("--").args(&owned).output().map_err(spawn)?;
        if !ls.status.success() {
            return Err(failed("git ls-files", &ls));
        }
        let tracked: std::collections::HashSet<&str> = std::str::from_utf8(&ls.stdout)
            .unwrap_or("")
            .split('\0')
            .filter(|s| !s.is_empty())
            .collect();
        owned.retain(|o| tracked.contains(o.as_str()) || vault.join(o).exists());
    }
    if owned.is_empty() {
        return Ok(false); // nothing of ours changed
    }
    // `-A` so a note deleted through the app is staged as a deletion, not left behind.
    let add = git(vault).arg("add").arg("-A").arg("--").args(&owned).output().map_err(spawn)?;
    if !add.status.success() {
        return Err(failed("git add", &add));
    }

    // **Small attachments, if this vault asked for them.** Off unless `vault.json` sets
    // `git_assets_max`, which is why the default behaviour is exactly what it always was: notes
    // travel, media does not.
    //
    // `-f` is required and is the whole trick: `ensure_repo` puts `blobs/` in `.gitignore`, and an
    // ignored path is skipped by a plain `add`. Git cannot filter by size itself, so the selection
    // happens here and each chosen file is named explicitly. Nothing else can slip in.
    let blobs = blobs_within(vault)?;
    if !blobs.is_empty() {
        let add = git(vault)
            .arg("add")
            .arg("-f")
            .arg("--")
            .args(&blobs)
            .output()
            .map_err(spawn)?;
        if !add.status.success() {
            return Err(failed("git add (assets)", &add));
        }
        owned.extend(blobs);
    }
    // `status` reported the *repo* dirty, which in a project vault is usually someone
    // else's work. Ask what actually landed in the index, and keep only the paths under a
    // directory of ours — the index may also hold something the user staged themselves,
    // and that is precisely what must not ride along.
    //
    // Committing these exact paths rather than the directories is not a detail: `git commit
    // --only -- notes` *fails* when `notes/` exists on disk but holds nothing git has ever
    // seen (git cannot track an empty directory), which is every vault before its first
    // note. A staged path is by definition one git can name.
    let staged = git(vault).args(["diff", "--cached", "--name-only"]).output().map_err(spawn)?;
    let ours: Vec<String> = String::from_utf8_lossy(&staged.stdout)
        .lines()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter(|p| owned.iter().any(|o| p == o || p.starts_with(&format!("{o}/"))))
        .map(String::from)
        .collect();
    if ours.is_empty() {
        return Ok(false);
    }
    // `--only` with explicit paths: commit the index we just built for these paths and
    // nothing else, so a file the user had staged elsewhere stays staged rather than being
    // swept into our commit.
    let out = git(vault)
        .arg("commit")
        .arg("--only")
        .arg("-m")
        .arg(message)
        .arg("--")
        .args(&ours)
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git commit", &out));
    }
    Ok(true)
}

/// Where the vault pushes to, or None when no remote is configured yet. Absence
/// is the normal state of a fresh vault, never an error.
pub fn remote(vault: &Path) -> Result<Option<String>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(None);
    }
    let out = git(vault).args(["remote", "get-url", REMOTE]).output().map_err(spawn)?;
    if !out.status.success() {
        return Ok(None); // "no such remote" is absence, not failure
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((!url.is_empty()).then_some(url))
}

/// Point `origin` at `url`, creating it when it doesn't exist yet. Idempotent,
/// so the panel can simply save whatever the user typed.
pub fn set_remote(vault: &Path, url: &str) -> Result<(), StoreError> {
    // `git remote add origin ""` *succeeds*, and the resulting remote then
    // reports its own name as its URL — so an empty value would leave the vault
    // claiming a push destination it does not have. Refuse it here: a backup that
    // lies about where it went is the one failure worth being strict about.
    let url = url.trim();
    ensure_repo(vault)?;
    // A remote is the moment this vault stops being private: from here on every
    // commit carries a name to somebody else's clone, and git history is forever.
    // If that name is still the placeholder, then every "who touched this?" the
    // product can ever answer is the same fake — so ask now, once, while a human is
    // looking at the panel that sent us here. Anyone whose git is already configured
    // never sees this.
    check_remote_allowed(vault, url, identity(vault).is_some())?;
    let sub = if remote(vault)?.is_some() { "set-url" } else { "add" };
    let out = git(vault).args(["remote", sub, REMOTE]).arg(url).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git remote", &out));
    }
    Ok(())
}

fn rev_parse(vault: &Path, rev: &str) -> Result<String, StoreError> {
    let out = git(vault).args(["rev-parse", rev]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-parse", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn branch(vault: &Path) -> Result<String, StoreError> {
    let out = git(vault).args(["rev-parse", "--abbrev-ref", "HEAD"]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-parse", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The remote-tracking ref for the current branch — `Some` only once we have
/// pushed at least once. `None` therefore means "never pushed", which is the
/// signal [`push_squashed`] uses to refuse to squash.
fn tracking(vault: &Path) -> Result<Option<String>, StoreError> {
    // An unborn HEAD (no commits yet) has no branch to track.
    let Ok(b) = branch(vault) else { return Ok(None) };
    let r = format!("refs/remotes/{REMOTE}/{b}");
    let ok = git(vault)
        .args(["rev-parse", "--verify", "--quiet", &r])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    Ok(ok.then_some(r))
}

/// Does `maybe_ancestor` lead to `descendant`? The one question that separates
/// "we are simply ahead" from "our histories have forked".
fn is_ancestor(vault: &Path, maybe_ancestor: &str, descendant: &str) -> Result<bool, StoreError> {
    let out = git(vault)
        .args(["merge-base", "--is-ancestor", maybe_ancestor, descendant])
        .output()
        .map_err(spawn)?;
    // Exit 1 means "no"; anything else is a real failure we should not read as "no".
    match out.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(failed("git merge-base", &out)),
    }
}

fn count_ahead(vault: &Path, base: &str) -> Result<u32, StoreError> {
    let out =
        git(vault).args(["rev-list", "--count", &format!("{base}..HEAD")]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-list", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0))
}

/// How many commits exist here but not on the remote. `None` when there is
/// nothing to compare against (no repo, no commits, or never pushed) — the
/// caller shows a count only when there is a real one.
pub fn unpushed(vault: &Path) -> Result<Option<u32>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(None);
    }
    match tracking(vault)? {
        None => Ok(None),
        Some(base) => Ok(Some(count_ahead(vault, &base)?)),
    }
}

/// One note's most-recent edit, exactly as git records it: who touched it, and when. This is the
/// whole collaboration read-model — authorship labels, the activity stream, the contributor
/// filter all come from here, because a note file is `notes/<ULID>.md`, so a changed path's stem
/// *is* the note id. Nothing is stored: git already knows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Touch {
    pub id: String,
    pub author: String,
    pub email: String,
    /// The author date as git's `%aI` (strict ISO-8601), for display and ordering.
    pub time: String,
}

/// Every note touched since `since` (a git `--since` value, e.g. `"1 year ago"`), each with its
/// **last** edit, most-recent-first. Read-only — one `git log`. Empty when there is no repo or no
/// history: authorship is a git capability, and its absence is not an error.
///
/// `--no-merges` because a merge commit's author is whoever *ran* the merge, not who wrote the
/// text. Fields are joined by `\x1f` (unit separator) so a name or email with spaces survives,
/// and each commit header is marked with a leading `\x01` so it can't be mistaken for a path.
pub fn activity(vault: &Path, since: &str) -> Result<Vec<Touch>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    let out = git(vault)
        .args([
            "log",
            "--no-merges",
            &format!("--since={since}"),
            "--pretty=format:\x01%an\x1f%ae\x1f%aI",
            "--name-only",
        ])
        .output()
        .map_err(spawn)?;
    // A brand-new repo with no commits exits non-zero on `log`; that is "no history", not failure.
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut touches = Vec::new();
    let mut cur: Option<(String, String, String)> = None; // (author, email, time)
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('\x01') {
            let mut f = rest.split('\x1f');
            cur = Some((
                f.next().unwrap_or_default().to_string(),
                f.next().unwrap_or_default().to_string(),
                f.next().unwrap_or_default().to_string(),
            ));
        } else if !line.is_empty() {
            // A changed path under the current commit. Only notes; first-seen (newest) wins.
            if let Some(id) = note_id_from_path(line) {
                if seen.insert(id.clone()) {
                    if let Some((author, email, time)) = cur.clone() {
                        touches.push(Touch { id, author, email, time });
                    }
                }
            }
        }
    }
    Ok(touches)
}

/// `notes/<ULID>.md` → `<ULID>`; blobs, manifest, `.view` files and anything nested → `None`.
fn note_id_from_path(path: &str) -> Option<String> {
    let stem = path.strip_prefix("notes/")?.strip_suffix(".md")?;
    (!stem.is_empty() && !stem.contains('/')).then(|| stem.to_string())
}

/// Collapse the not-yet-pushed commits into one and push. Returns how many were
/// squashed (0 when there was nothing to squash, or on the first push).
///
/// Auto-commit produces a `auto:` commit every few seconds of editing; without
/// this the remote would accrue thousands of them. Squashing costs the granular
/// undo for the squashed window — after a push you can only step back to that
/// push — which is the accepted trade (see docs/context/decisions.md).
pub fn push_squashed(vault: &Path, message: &str) -> Result<u32, StoreError> {
    ensure_repo(vault)?;
    if remote(vault)?.is_none() {
        return Err(StoreError::Io(
            "no remote configured — set one to push your notes off this machine".into(),
        ));
    }
    // Where the history stood before we collapsed it. The squash is a bet that the
    // push lands; if it doesn't, this is what we put back.
    let head_before = rev_parse(vault, "HEAD").ok();
    let squashed = match tracking(vault)? {
        // The first push. Here "unpushed" means the *entire* history, and
        // destroying history that has never left the machine is exactly
        // backwards — send it as it stands. Every later push collapses to one.
        None => 0,
        Some(tracked) => {
            // **Squash only what we wrote.** `reset --soft <tracking>` collapses *every*
            // unpushed commit — and the justification for squashing at all is that the app
            // auto-commits every few seconds. That justifies collapsing **ours**; it never
            // justified collapsing three hand-written manuscript commits into one `backup:`,
            // which is what happened the moment a vault was also a repo you commit to
            // yourself.
            //
            // Discriminated by the **message prefix, never the author**: a personal machine
            // has one git user and we commit *as* them, so author-based detection cannot
            // work here — it would classify every commit as ours.
            //
            // A dedicated vault has only `auto:`/`backup:` commits, so `base` comes out as
            // the tracking ref and the behaviour is bit-identical to before. No mode, no
            // flag — derived from the history itself.
            let base = newest_foreign(vault, &tracked)?.unwrap_or_else(|| tracked.clone());
            // Squash ONLY when the remote's tip is an ancestor of ours — i.e. we
            // hold everything it holds. Otherwise someone else's commits are on
            // that ref (something fetched), and `reset --soft` onto it would put
            // *their* tip under *our* tree: a commit that deletes their work and
            // then pushes as a clean fast-forward. Nothing rejects it.
            //
            // A count of unpushed commits cannot catch this — it is >0 in exactly
            // the divergent case, so it reads as "normal". Ancestry is the question;
            // "how many" never was.
            // Still asked about the *tracking* ref: the question is "does the remote hold
            // something we don't", which our own squash boundary has no opinion on.
            if !is_ancestor(vault, &tracked, "HEAD")? {
                return Err(StoreError::Io(
                    "the remote has changes you don't have — pull first, then back up".into(),
                ));
            }
            let n = count_ahead(vault, &base)?;
            if n > 1 {
                // --soft moves HEAD alone: the index still holds every squashed
                // change, so the commit below reproduces the same tree.
                let out = git(vault).args(["reset", "--soft", &base]).output().map_err(spawn)?;
                if !out.status.success() {
                    return Err(failed("git reset", &out));
                }
                // A net-zero window (write something, then undo it) leaves an
                // index identical to the remote's tree. Committing that would
                // fail and strand HEAD mid-squash, so skip it: we are already at
                // the remote's state and the push below is simply a no-op.
                let status = git(vault).args(["status", "--porcelain"]).output().map_err(spawn)?;
                if !status.stdout.is_empty() {
                    let out =
                        git(vault).arg("commit").arg("-m").arg(message).output().map_err(spawn)?;
                    if !out.status.success() {
                        return Err(failed("git commit", &out));
                    }
                }
                n
            } else {
                0
            }
        }
    };
    // -u so the tracking ref exists next time, which is what makes the squash
    // above possible at all.
    let out = git(vault).args(["push", "-u", REMOTE, "HEAD"]).output().map_err(spawn)?;
    if !out.status.success() {
        // The squash was a bet that the push would land. It didn't — most likely
        // the remote moved and rejected us, which is the very case this function
        // exists to make safe. Put the history back: leaving it collapsed would
        // charge the user their granular undo for a backup that never happened.
        // `--soft` restores HEAD without touching the index, which already holds
        // this tree, so the working tree is untouched either way.
        if squashed > 0 {
            if let Some(h) = &head_before {
                let _ = git(vault).args(["reset", "--soft", h]).output();
            }
        }
        return Err(failed("git push", &out));
    }

    // The push says it worked. **Ask the remote, not the pusher.**
    //
    // With subprocess git this is belt and braces: git's exit code is trustworthy, so this
    // never fires. It is here for the client that replaces it. The moment push is spoken by
    // our own code — a hand-written `send-pack`, or a library whose error mapping we own —
    // "success" becomes our own parser's opinion, and a *false* success is the one failure
    // this function cannot survive: the squash already collapsed the user's granular history
    // on the strength of it, so they pay their undo for a backup that never happened. Every
    // other check here (ancestry, net-zero window) is about not destroying someone else's
    // work; this one is about not destroying our own on a lie.
    //
    // Only when something was squashed. With `squashed == 0` nothing was collapsed, so a
    // false success costs an unpushed vault — which `backup_status` already surfaces — rather
    // than lost history, and it is not worth a round trip on the common path.
    if squashed > 0 {
        if let (Some(head), Some(branch)) = (rev_parse(vault, "HEAD").ok(), current_branch(vault)) {
            // A *definite* mismatch, and nothing else. If the remote cannot be asked — it went
            // away between the push and now, or does not publish this branch — the answer is
            // unknown, and unknown must not roll back: undoing a push that actually landed
            // leaves local behind a remote that already has the work, so every later push is
            // rejected as divergent. Failing to verify is not the same as failing to push.
            if let Ok(Some(there)) = remote_head(vault, &branch) {
                if there != head {
                    if let Some(h) = &head_before {
                        let _ = git(vault).args(["reset", "--soft", h]).output();
                    }
                    return Err(StoreError::Io(format!(
                        "the push reported success but {REMOTE} still points at {} — your \
                         history has been put back the way it was, and nothing was backed up",
                        &there[..there.len().min(8)]
                    )));
                }
            }
        }
    }
    Ok(squashed)
}

/// The branch we are on, or `None` when there isn't one to name (a detached HEAD, which
/// `push -u … HEAD` would refuse anyway). Used to ask the remote about the right ref.
fn current_branch(vault: &Path) -> Option<String> {
    let out = git(vault).args(["rev-parse", "--abbrev-ref", "HEAD"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!name.is_empty() && name != "HEAD").then_some(name)
}

/// What the remote says its branch points at, straight from the wire — `None` when it does
/// not have that branch at all, which is an honest "cannot tell", not a mismatch.
fn remote_head(vault: &Path, branch: &str) -> Result<Option<String>, StoreError> {
    let out = git(vault)
        .args(["ls-remote", REMOTE, &format!("refs/heads/{branch}")])
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git ls-remote", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string))
}

/// What a [`pull`] did. Every arm is a thing the user needs told differently, which is
/// why this is not a bool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pulled {
    /// The remote had nothing we don't. The overwhelmingly common case.
    UpToDate,
    /// Their work arrived and merged, cleanly. `0` when we simply fast-forwarded.
    Merged(u32),
    /// Their work arrived and genuinely disagrees with ours. The named notes have
    /// conflict markers in them and are waiting for a human. Thanks to the `.md` merge
    /// driver those markers are in the *body*, so the notes still open in the editor.
    Conflicted(Vec<String>),
}

/// Has the remote moved? A `ls-remote` against the configured remote, compared with our
/// tracking ref — one cheap network round-trip that touches no refs and writes nothing,
/// so it is safe to ask on a timer.
///
/// `None` when there is nothing to compare (no remote, or never pushed). Deliberately
/// **not** a fetch: fetching is what advances the tracking ref, and an advanced tracking
/// ref is what [`push_squashed`]'s ancestry guard exists to survive. Ask first, fetch on
/// purpose.
pub fn remote_moved(vault: &Path) -> Result<Option<bool>, StoreError> {
    if !vault.join(".git").exists() || remote(vault)?.is_none() {
        return Ok(None);
    }
    let Ok(b) = branch(vault) else { return Ok(None) };
    let Some(track) = tracking(vault)? else { return Ok(None) };

    let out = git(vault).args(["ls-remote", REMOTE, &format!("refs/heads/{b}")]).output().map_err(spawn)?;
    if !out.status.success() {
        // Offline, or no such branch there yet. Not knowing is not an error: this runs
        // on a timer and a laptop that sleeps must not show the user a failure.
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let Some(theirs) = stdout.split_whitespace().next() else { return Ok(Some(false)) };
    Ok(Some(rev_parse(vault, &track).map(|ours| ours != theirs).unwrap_or(false)))
}

/// Fetch and merge the remote's work into ours — the other half of a shared vault, and
/// the reason every guard above it had to land first.
///
/// Merging runs the `.md` driver installed by [`ensure_repo`], so two people editing
/// different paragraphs of one note is a non-event rather than a conflict on the
/// `updated:` line we rewrite on every save.
pub fn pull(vault: &Path) -> Result<Pulled, StoreError> {
    ensure_repo(vault)?;
    if remote(vault)?.is_none() {
        return Err(StoreError::Io(
            "no remote configured — set one before pulling anyone's work".into(),
        ));
    }
    if unmerged_paths(vault)?.is_some() {
        return Err(StoreError::Io(
            "there is already a merge to finish here — resolve the conflicts first".into(),
        ));
    }
    let out = git(vault).args(["fetch", REMOTE]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git fetch", &out));
    }
    let Some(track) = tracking(vault)? else {
        // Nothing of ours is up there yet, so there is nothing of theirs to merge into.
        return Ok(Pulled::UpToDate);
    };
    // Already hold everything they have: no merge, no commit, nothing to say.
    if is_ancestor(vault, &track, "HEAD")? {
        return Ok(Pulled::UpToDate);
    }
    let incoming = count_range(vault, "HEAD", &track)?;

    let out = git(vault).args(["merge", "--no-edit", &track]).output().map_err(spawn)?;
    if out.status.success() {
        return Ok(Pulled::Merged(incoming));
    }
    // A merge that stopped is either a real conflict — which is a *result*, not a
    // failure — or something else entirely, which is.
    match unmerged_paths(vault)? {
        Some(files) => Ok(Pulled::Conflicted(files)),
        None => Err(failed("git merge", &out)),
    }
}

/// The newest unpushed commit this app did **not** write, or `None` when every unpushed
/// commit is ours.
///
/// The squash boundary. `push_squashed` may collapse the window of `auto:`/`backup:`
/// commits the debounced auto-commit produces — that is what it exists for — but a commit
/// the user wrote by hand is a unit of *their* history and collapsing it is data loss of
/// the quiet kind: the work survives, its shape does not.
///
/// **By message prefix, deliberately not by author.** We commit as the user's own git
/// identity (that is the point of `ensure_identity`), so on a personal machine every commit
/// has the same author and an author test classifies everything as ours. The prefix is the
/// only signal that actually distinguishes them, and it is one we control on write.
fn newest_foreign(vault: &Path, tracked: &str) -> Result<Option<String>, StoreError> {
    let out = git(vault)
        .args(["log", "--format=%H %s", &format!("{tracked}..HEAD")])
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git log", &out));
    }
    // Newest first, which is the order we want: the first foreign commit we meet walking
    // back from HEAD is the floor the squash must not go below.
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some((hash, subject)) = line.split_once(' ') else { continue };
        if !subject.starts_with("auto:") && !subject.starts_with("backup:") {
            return Ok(Some(hash.to_string()));
        }
    }
    Ok(None)
}

/// How many commits `to` has that `from` does not.
fn count_range(vault: &Path, from: &str, to: &str) -> Result<u32, StoreError> {
    let out = git(vault)
        .args(["rev-list", "--count", &format!("{from}..{to}")])
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-list", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0))
}

/// The paths git considers unmerged, or `None` when the tree is not mid-conflict.
/// `pub` because the conflict list is something the product has to be able to show:
/// a note with markers in it is the one state a user must be told about by name.
pub fn conflicts(vault: &Path) -> Result<Vec<String>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    Ok(unmerged_paths(vault)?.unwrap_or_default())
}

fn unmerged_paths(vault: &Path) -> Result<Option<Vec<String>>, StoreError> {
    let status = git(vault).args(["status", "--porcelain"]).output().map_err(spawn)?;
    if !status.status.success() {
        return Err(failed("git status", &status));
    }
    let files: Vec<String> = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|l| unmerged(l))
        .map(|l| l[3..].trim().to_string())
        .collect();
    Ok((!files.is_empty()).then_some(files))
}

/// Is this `status --porcelain` line an unmerged path? The seven conflict codes
/// are `DD AU UD UA DU AA UU` — every one has a `U`, except the two doubles.
fn unmerged(line: &str) -> bool {
    let mut c = line.chars();
    match (c.next(), c.next()) {
        (Some('U'), _) | (Some(_), Some('U')) => true,
        (Some('A'), Some('A')) | (Some('D'), Some('D')) => true,
        _ => false,
    }
}

fn spawn(e: std::io::Error) -> StoreError {
    StoreError::Io(format!("could not run git (is it installed?): {e}"))
}

fn failed(what: &str, out: &Output) -> StoreError {
    let stderr = String::from_utf8_lossy(&out.stderr);
    StoreError::Io(format!("{what} failed: {}", stderr.trim()))
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

/// A vault-relative path string for git, or `None` when the path is not inside the vault.
///
/// git pathspecs are resolved against the repo root, and `commit --only` refuses one it
/// cannot match — so an absolute path from another vault would fail the whole commit rather
/// than being ignored.
fn relative(vault: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(vault).ok().map(|p| p.to_string_lossy().into_owned())
}

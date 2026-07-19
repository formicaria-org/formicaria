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
const REMOTE: &str = "origin";

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
const PLACEHOLDER_NAME: &str = "formicaria";
const PLACEHOLDER_EMAIL: &str = "formicaria@localhost";

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
    if url.trim().is_empty() {
        return Err(StoreError::Io("a clone needs a remote URL".into()));
    }
    // `git clone` refuses a non-empty target on its own, but says it in terms of the
    // directory rather than of what the user was doing — and by then it has already created
    // the directory when it did not exist. Refuse first, and write nothing.
    if dest.exists() && dest.read_dir().is_ok_and(|mut d| d.next().is_some()) {
        return Err(StoreError::Io(format!(
            "{} already exists and is not empty — clone into a new directory",
            dest.display()
        )));
    }

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
    ensure_line(&vault.join(".gitattributes"), "*.md merge=fm text eol=lf")
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
    let Some(exe) = merge_command() else { return Ok(()) };
    for (key, value) in [
        ("merge.fm.name", "formicaria frontmatter-aware note merge".to_string()),
        // %O base, %A ours (and where the answer goes), %B theirs, %L marker size.
        ("merge.fm.driver", format!("'{exe}' merge-md %O %A %B %L")),
    ] {
        let out = git(vault).args(["config", &key, &value]).output().map_err(spawn)?;
        if !out.status.success() {
            return Err(failed("git config", &out));
        }
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

/// Who this vault's commits are attributed to, or `None` when nobody real is —
/// either git has no identity at all, or it still holds our placeholder. Reporting
/// our own stand-in as absent is deliberate: it is the difference between a vault
/// that asks once and a vault that quietly signs a shared history `formicaria`.
pub fn identity(vault: &Path) -> Option<Identity> {
    if !vault.join(".git").exists() {
        return None;
    }
    let email = config(vault, "user.email")?;
    if email == PLACEHOLDER_EMAIL {
        return None;
    }
    Some(Identity { name: config(vault, "user.name")?, email })
}

/// Record who the user is, repo-locally — the answer to the question
/// [`set_remote`] asks. Scoped to this vault because a vault is an audience: the
/// name you push to a lab repo need not be the one on your personal notes, and this
/// app has no business editing anyone's global git config.
pub fn set_identity(vault: &Path, name: &str, email: &str) -> Result<(), StoreError> {
    let (name, email) = (name.trim(), email.trim());
    if name.is_empty() || email.is_empty() {
        return Err(StoreError::Io("a git identity needs both a name and an email".into()));
    }
    // Not validation — git does none either, and an address this app rejects is an
    // address the user cannot use. This catches only the mistake that is invisible
    // afterwards: a name typed into the email box, signed into history forever.
    if !email.contains('@') || email == PLACEHOLDER_EMAIL {
        return Err(StoreError::Io(format!("'{email}' is not an email address")));
    }
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
    if owned.is_empty() {
        return Ok(false); // nothing of ours changed
    }
    // `-A` so a note deleted through the app is staged as a deletion, not left behind.
    let add = git(vault).arg("add").arg("-A").arg("--").args(&owned).output().map_err(spawn)?;
    if !add.status.success() {
        return Err(failed("git add", &add));
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
    if url.is_empty() {
        return Err(StoreError::Io("a remote needs a URL".into()));
    }
    ensure_repo(vault)?;
    // A remote is the moment this vault stops being private: from here on every
    // commit carries a name to somebody else's clone, and git history is forever.
    // If that name is still the placeholder, then every "who touched this?" the
    // product can ever answer is the same fake — so ask now, once, while a human is
    // looking at the panel that sent us here. Anyone whose git is already configured
    // never sees this.
    if identity(vault).is_none() {
        return Err(StoreError::Io(
            "tell us who you are first — your name and email sign every commit you share".into(),
        ));
    }
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
    Ok(squashed)
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

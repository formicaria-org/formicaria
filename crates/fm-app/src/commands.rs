//! The command surface — one pure function per IPC call, each over the [`Store`]
//! seam so it is testable with `MemoryStore` and identical against `FileStore`.
//! `fm_app::dispatch` is the one surface that wraps these; a transport only frames them. Nothing here knows what HTTP is.

use crate::dto::{value_string, Board, Column, NoteDetail, ObjectMeta};
use crate::refs;
use fm_core::{apply_property, ingest, BlobStore, Manifest, Store, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query, SortKey};
use serde::Serialize;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use ulid::Ulid;

/// Group every note by `group_by` into board columns, newest card first. The
/// property is opaque: pass `"status"` for a status board, or any custom key —
/// no code changes.
///
/// Assets are excluded: an asset is a blob a note *references*, not something you
/// plan, so it has no place in a column. Find one via `search` (its extracted
/// text is indexed) or via the note that references it. Same rule in [`agenda`]
/// and [`recent`]; `search` and [`gallery`] deliberately still see assets.
pub fn board(store: &dyn Store, group_by: &str) -> Result<Board, StoreError> {
    let q = Query {
        filter: crate::thread::notes_base(),
        group_by: Some(group_by.to_string()),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    let res = store.query(&q)?;
    let columns = res
        .groups
        .unwrap_or_default()
        .into_iter()
        .map(|g| Column {
            value: value_string(&g.key),
            label: g.label,
            cards: g.rows.iter().map(ObjectMeta::from).collect(),
        })
        .collect();
    Ok(Board { group_by: group_by.to_string(), columns })
}

/// The gallery: every asset, newest first. This is the S4 checkpoint — a
/// *second* renderer that is nothing but a different query (`type = asset`) over
/// the same engine and the same `ObjectMeta`. No new query machinery, no new
/// storage path; if this were expensive, the `Store`/`fm-query` seam would not
/// be real. Thumbnails and lazy loading arrive with S5's ingest; until a blob
/// exists, the tile shows the shared "asset not found" placeholder.
pub fn gallery(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![Kind::Asset])),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// The agenda: the closest-deadline view. Everything with a `due` date that is
/// not done, soonest first. This is "zero new code" — it is the same engine and
/// the same `ObjectMeta`, just a different filter and sort; urgency is *derived*
/// in the card from `due`, never stored (there is no priority field).
///
/// "done" is the completion convention: a note with no status is not done, so it
/// is included (`status != done` keeps nulls). A meeting carries a date, so it
/// shows up here too. This filter is the one place the string "done" is written;
/// it stays out of the renderers (a `.view` file could override it).
pub fn agenda(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: crate::thread::notes_base()
            .and(Predicate::Prop {
                key: "due".into(),
                op: Op::Exists,
                value: PropertyValue::Null,
            })
            .and(Predicate::Prop {
                key: "status".into(),
                op: Op::Ne,
                value: PropertyValue::Text("done".into()),
            }),
        sort: vec![SortKey::asc("due")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Fetch one note with its full body — the read view's payload. The list
/// commands return meta only; the body crosses IPC only when a note is opened.
pub fn get(store: &dyn Store, id: &str) -> Result<Option<NoteDetail>, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    Ok(store
        .get(id)?
        .map(|o| NoteDetail {
            meta: ObjectMeta::from(&o),
            version: crate::dto::version_of(&o.body),
            body: o.body.clone(),
        }))
}

/// Capture a note; the text becomes the body. `vault` is the audience it joins —
/// empty means the default vault, an unknown name is refused by `Store::put`'s
/// routing (same discipline as `ingest`). Returns the new card's meta so the UI
/// can slot it into the board without a full refetch.
/// A note's citation as a BibTeX entry — the thing a user pastes into a manuscript.
///
/// Server-side so the format has **one** implementation: `paper::parse_bibtex` reads it and
/// `paper::to_bibtex` writes it, and the round trip is tested. Computing it in the UI would put a
/// second, drifting copy in TypeScript.
pub fn paper_bibtex(store: &dyn Store, id: &str) -> Result<String, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let obj = store.get(id)?.ok_or_else(|| StoreError::Parse("no such note".into()))?;
    let props: std::collections::BTreeMap<String, String> = obj
        .extra
        .iter()
        .map(|(k, v)| (k.clone(), v.display()))
        .collect();
    Ok(crate::paper::to_bibtex(obj.title.as_deref().unwrap_or_default(), &props))
}

/// **Create a paper from whatever the user has to hand** — a BibTeX entry, an identifier, a URL,
/// or just a title.
///
/// A paper is a `Kind::Note` tagged `paper`, never a `Kind::Asset`: `views.rs`'s base query
/// hardcodes `Kind(Note)` for board/agenda/timeline, so an asset could appear in no planning view,
/// and the paper — not the PDF — is the thing you tag, schedule and think about. The PDF is a blob
/// the note references, attached separately.
///
/// **One `put`, not a create plus a dozen `set_property` calls.** Every field is applied to the
/// object in memory and written once, so a failure cannot leave a paper with a title, a year and
/// nothing else — and importing a library does not become tens of thousands of round trips, each a
/// full file rewrite plus an index update.
///
/// Values go through [`apply_property`], so a hand-pasted `year` is typed exactly as a
/// hand-*edited* file would type it — the invariant that stops one vault sorting into two blocks.
pub fn create_paper(
    store: &mut dyn Store,
    input: &str,
    vault: &str,
) -> Result<ObjectMeta, StoreError> {
    use crate::paper::{parse_bibtex, parse_identifier, PaperFields};

    let input = input.trim();
    // BibTeX first: it is the only input that carries a whole record, so it wins where both parse.
    let mut fields = parse_bibtex(input).unwrap_or_default();
    if fields.is_empty() {
        if let Some(id) = parse_identifier(input) {
            let v = Some(id.value().to_string());
            match id {
                crate::paper::Identifier::Doi(_) => fields.doi = v,
                crate::paper::Identifier::ArXiv(_) => fields.arxiv = v,
                crate::paper::Identifier::Url(_) => fields.url = v,
            }
        } else if !input.is_empty() {
            // Not a citation and not an identifier: the user typed a title, which is a perfectly
            // good way to start a paper note and the only one that always works offline.
            fields = PaperFields { title: Some(input.to_string()), ..Default::default() };
        }
    }

    let mut obj = Object::new(Kind::Note, "");
    obj.vault = vault.to_string();
    obj.tags = vec!["paper".to_string()];
    obj.title = fields.title.clone().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    for (key, value) in fields.properties() {
        apply_property(&mut obj, key, &value)?;
    }
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

pub fn capture(store: &mut dyn Store, body: &str, vault: &str) -> Result<ObjectMeta, StoreError> {
    let mut obj = Object::new(Kind::Note, body);
    obj.vault = vault.to_string();
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

/// Notes nothing has touched since `since` — **derived from git, stored nowhere.**
///
/// Ruling 15 settles the mechanism before the feature is designed: *"Presence/status →
/// **derived** from git log or pushed by the peer, **stored nowhere**. … Status is not
/// knowledge."* A `stale: true` property would be the exact violation: it would be wrong the
/// moment you edited the note, it would travel into a collaborator's clone as an opinion, and
/// it would be one more thing to keep true. Git already knows when each file last changed, so
/// this asks it — the same "collaboration is git, *exposed*" stance `activity` takes, pointed
/// at absence instead of presence.
///
/// `since` is a git `--since` value (`"90 days ago"`), so the threshold is the caller's and
/// nothing is configured, stored, or defaulted on disk.
///
/// **Reports, never acts.** Same discipline as `verify`: naming an old note is awareness;
/// deciding what to do about it is the human's. Nothing here archives, deletes, or flags.
///
/// Messages are excluded with everything else via [`crate::thread::notes_base`] — a five-word
/// reply from March is not a stale note, it is a finished sentence.
pub fn stale(
    store: &dyn Store,
    vault_path: &Path,
    since: &str,
) -> Result<Vec<ObjectMeta>, StoreError> {
    // **Refuse rather than mislead.** Without history there is no evidence of staleness, and
    // both silent answers are lies: an empty list says "nothing is old", a full one says
    // "everything is". Git is a capability this product declares, not one it assumes.
    if !vault_path.join(".git").exists() {
        return Err(StoreError::Io(
            "staleness is read from git history, and this vault has none yet".into(),
        ));
    }
    let touched: std::collections::HashSet<String> = fm_core::vcs::activity(vault_path, since)?
        .into_iter()
        .map(|t| t.id)
        .collect();

    let q = Query {
        filter: crate::thread::notes_base(),
        // Oldest first: the answer to "what have I not looked at" wants the worst offender at
        // the top. `updated` is the note's own claim; git is what decided it was stale.
        sort: vec![SortKey::asc("updated")],
        ..Default::default()
    };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| !touched.contains(&o.id.to_string()))
        .map(ObjectMeta::from)
        .collect())
}

/// One message in a discussion. `depth` is derived, never stored.
#[derive(Serialize)]
pub struct Message {
    #[serde(flatten)]
    pub meta: ObjectMeta,
    pub body: String,
    /// The message this answers, as a plain id — `None` when it answers the root, or when the
    /// stored pointer is unusable (dangling, circular, hand-mangled).
    pub reply_to: Option<String>,
    /// Indentation, computed here so the two frontends cannot each invent it. Capped, and
    /// cycle-safe: see [`thread`].
    pub depth: u8,
}

/// A note's discussion: the root (absent if it was deleted) and its messages, oldest first.
#[derive(Serialize)]
pub struct ThreadView {
    pub root: Option<ObjectMeta>,
    pub count: usize,
    pub messages: Vec<Message>,
}

/// How deep a reply may be shown. Beyond this the indent stops growing — a hand-edited chain
/// forty deep must not push a pane off the side of the screen.
const MAX_DEPTH: u8 = 4;

/// Post a message to a note's discussion — **one file write.**
///
/// It is its own command rather than a composition of existing ones because `capture` takes a
/// body and a vault and nothing else: a reply built from it would be `capture` + two
/// `set_property` calls, i.e. three whole-file rewrites and three index updates to post one
/// line, with no transaction around them (`MASTERPLAN.md`: *no atomic multi-object
/// transactions*).
///
/// **Replying to a message re-roots.** `thread_of` always names the root note, never another
/// message — so a reply-to-a-reply joins the same discussion instead of starting an invisible
/// one hanging off a note that no view can reach. The parent is preserved in `reply_to`, which
/// is what "reply to this comment" actually means. Getting this backwards is how the first
/// attempt made the natural gesture the broken one.
///
/// The message joins the **target's vault**, never a caller-chosen one. A vault is an
/// audience; a reply that landed elsewhere would be invisible to the discussion or visible to
/// people the note was never shared with, and that is not a choice to leave to a UI.
pub fn reply(store: &mut dyn Store, target: &str, body: &str) -> Result<ObjectMeta, StoreError> {
    let target_id: Id =
        target.parse().map_err(|_| StoreError::Parse(format!("invalid id: {target}")))?;
    let target_obj = store.get(target_id)?.ok_or(StoreError::NotFound(target_id))?;
    if target_obj.kind != Kind::Note {
        return Err(StoreError::Io("you can only reply to a note".into()));
    }
    if body.trim().is_empty() {
        return Err(StoreError::Io("a message needs something in it".into()));
    }

    // If the target is itself a message, its root is the discussion; otherwise the target is
    // the root. A malformed `thread_of` on the target falls through to "the target is a root",
    // so garbage is never propagated into a new file.
    let root = match target_obj.get(crate::thread::THREAD_OF) {
        PropertyValue::Text(s) => fm_model::parse_note_ref(&s).unwrap_or(target_id),
        _ => target_id,
    };

    let mut obj = Object::new(Kind::Note, body);
    obj.vault = target_obj.vault.clone();
    obj.extra
        .insert(crate::thread::THREAD_OF.into(), PropertyValue::Text(fm_model::note_ref(root)));
    obj.extra
        .insert(crate::thread::REPLY_TO.into(), PropertyValue::Text(fm_model::note_ref(target_id)));
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

/// One note's discussion, oldest first.
///
/// Sorted by `created`, which for ULID-named files is also creation order — the same sequence
/// on every machine, with no clock to disagree about.
///
/// **The reader is flat, and derives depth defensively.** `reply_to` is ordinary frontmatter a
/// human can hand-edit, so it can dangle (the parent was deleted) or cycle (`a → b → a`). A
/// recursive builder meets those as a silently dropped subtree and an infinite loop. Here every
/// message that exists is returned exactly once whatever its pointer says; a broken pointer
/// costs indentation, never visibility. An unreachable note is a failure mode this project
/// names by hand and refuses.
///
/// A deleted root yields `root: None` with the messages intact — the discussion is not
/// swallowed by its subject's deletion.
pub fn thread(store: &dyn Store, root: &str) -> Result<ThreadView, StoreError> {
    let root_id: Id = root.parse().map_err(|_| StoreError::Parse(format!("invalid id: {root}")))?;
    let q = Query {
        // No `Kind(Note)` conjunct, and that is deliberate rather than an oversight: carrying a
        // well-formed `thread_of` *is* what makes something a message, and only `reply` writes
        // one (which refuses a non-note target and creates a `Kind::Note`). Adding the kind
        // would be a second, weaker spelling of the same condition — and it would trip the CI
        // grep that keeps the notes-only base in one place, for a query that deliberately wants
        // the opposite of that base.
        filter: Filter::new().and(Predicate::NoteRef {
            key: crate::thread::THREAD_OF.into(),
            id: Some(root_id),
        }),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    // Exclude the root itself. It normally is not a match (an ordinary note has no `thread_of`),
    // but a **first-class discussion** is self-anchored (`thread_of: note:<own-id>`, see
    // [`crate::thread::is_discussion_root`]), so it would otherwise appear as the first message in
    // its own thread. One line, and it leaves ordinary note-rooted threads untouched.
    let rows: Vec<Object> = store.query(&q)?.rows.into_iter().filter(|o| o.id != root_id).collect();

    // Parent lookup over exactly the messages in this thread. A pointer to anything outside it
    // (the root itself, a deleted message, another vault) simply is not found → depth 0.
    let present: std::collections::HashMap<Id, usize> =
        rows.iter().enumerate().map(|(i, o)| (o.id, i)).collect();
    let parent_of = |o: &Object| -> Option<usize> {
        match o.get(crate::thread::REPLY_TO) {
            PropertyValue::Text(s) => fm_model::parse_note_ref(&s).and_then(|p| present.get(&p).copied()),
            _ => None,
        }
    };

    let messages = rows
        .iter()
        .map(|o| {
            // Walk to the root counting hops, refusing to visit any message twice. The visited
            // set is what makes a hand-edited cycle terminate; the cap is what stops a long
            // legitimate chain from indenting off-screen.
            let mut depth = 0u8;
            let mut seen = std::collections::HashSet::new();
            let mut cur = o;
            while let Some(i) = parent_of(cur) {
                if !seen.insert(cur.id) || depth >= MAX_DEPTH {
                    break;
                }
                depth += 1;
                cur = &rows[i];
            }
            Message {
                meta: ObjectMeta::from(o),
                body: o.body.clone(),
                reply_to: match o.get(crate::thread::REPLY_TO) {
                    PropertyValue::Text(s) => fm_model::parse_note_ref(&s).map(|i| i.to_string()),
                    _ => None,
                },
                depth,
            }
        })
        .collect::<Vec<_>>();

    Ok(ThreadView {
        root: store.get(root_id)?.as_ref().map(ObjectMeta::from),
        count: messages.len(),
        messages,
    })
}

/// Create a **first-class discussion** — a note that is the root of its own thread.
///
/// A dedicated command, not `capture` + `set_property`: `capture` sets only body and vault, and
/// `set_property` *refuses* `thread_of` (structure is not hand-settable — the same guard that stops
/// a board drag erasing a note). So the self-anchor is written here, once, under the app's control,
/// exactly as `reply` writes a message's pointers.
///
/// The `title` is what the Discussions view shows at a glance; the body is empty because a
/// discussion's content is the conversation, not a document.
pub fn create_discussion(
    store: &mut dyn Store,
    title: &str,
    vault: &str,
) -> Result<ObjectMeta, StoreError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(StoreError::Io("a discussion needs a title".into()));
    }
    let mut obj = Object::new(Kind::Note, "");
    obj.vault = vault.to_string();
    obj.title = Some(title.to_string());
    // Self-anchor: the discussion is the root of its own thread (see `thread::is_discussion_root`).
    // This is what makes it a discussion — and, being a well-formed `thread_of`, it is an
    // `is_message` note, so it drops out of every planning view for free.
    obj.extra
        .insert(crate::thread::THREAD_OF.into(), PropertyValue::Text(fm_model::note_ref(obj.id)));
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

/// A person who has posted in a discussion — `name` for display, `email` for the avatar hue. It is
/// [`fm_core::git::Identity`] because that is exactly what it is: git authorship (Ruling 14), the
/// same source as the "edited by" labels, derived and never stored.
pub type Participant = fm_core::git::Identity;

/// One discussion, as the Discussions view shows it at a glance.
#[derive(Serialize)]
pub struct DiscussionSummary {
    #[serde(flatten)]
    pub root: ObjectMeta,
    /// How many messages the discussion holds (not counting the root itself).
    pub count: usize,
    /// RFC-3339 of the newest message, or the discussion's own creation when empty — the sort key.
    pub last_activity: String,
    /// Who left a message, filled from git by the dispatcher (Ruling 14). Empty here (the pure
    /// `Store` layer has no authorship) and empty for a vault with no history yet — a discussion
    /// still shows its title and vault in that case.
    pub participants: Vec<Participant>,
}

/// Every first-class discussion, most-recently-active first — the Discussions view's feed.
///
/// A discussion is a self-rooted note (`thread_of` pointing at itself, [`crate::thread::is_discussion_root`]);
/// its messages are notes whose `thread_of` names it. This lists the roots and counts their
/// messages. **Comment threads hanging off an ordinary note are deliberately *not* here** — their
/// root is not a discussion, so they stay with their note (the per-note discussion panel), and this
/// view is only the discussions that "live on their own".
///
/// `participants` is left empty; the dispatcher fills it from each vault's git log, because
/// authorship is a git fact and this layer is storage-only. Same `candidates()`/`load_all()` cost
/// as `thread`/`recent`, already accepted — no index.
pub fn discussions(store: &dyn Store) -> Result<Vec<DiscussionSummary>, StoreError> {
    // Every message-class note in one pass: roots (self-anchored) and replies alike.
    let q = Query {
        filter: Filter::new()
            .and(Predicate::NoteRef { key: crate::thread::THREAD_OF.into(), id: None }),
        ..Default::default()
    };
    let msgs = store.query(&q)?.rows;

    use std::collections::HashMap;
    let mut roots: Vec<Object> = Vec::new();
    let mut count: HashMap<Id, usize> = HashMap::new();
    let mut last: HashMap<Id, OffsetDateTime> = HashMap::new();
    for o in &msgs {
        if crate::thread::is_discussion_root(o) {
            roots.push(o.clone());
            // A brand-new, empty discussion is at least as "recent" as its own creation.
            let e = last.entry(o.id).or_insert(o.created);
            if o.created > *e {
                *e = o.created;
            }
        } else if let PropertyValue::Text(s) = o.get(crate::thread::THREAD_OF) {
            if let Some(root) = fm_model::parse_note_ref(&s) {
                *count.entry(root).or_default() += 1;
                let e = last.entry(root).or_insert(o.created);
                if o.created > *e {
                    *e = o.created;
                }
            }
        }
    }

    let mut out: Vec<DiscussionSummary> = roots
        .iter()
        .map(|r| DiscussionSummary {
            root: ObjectMeta::from(r),
            count: count.get(&r.id).copied().unwrap_or(0),
            last_activity: last
                .get(&r.id)
                .copied()
                .unwrap_or(r.created)
                .format(&Rfc3339)
                .unwrap_or_default(),
            participants: Vec::new(),
        })
        .collect();
    // Most-recently-active first — the point of the view is "what is going on now".
    out.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    Ok(out)
}

/// A watched thread: a root note id and how many messages hang off it.
#[derive(Serialize)]
pub struct ThreadRoot {
    pub id: String,
    pub count: usize,
}

/// **Every** thread with messages — first-class discussions *and* ordinary notes that have a comment
/// thread. Unlike [`discussions`] (which lists only first-class discussion roots for the Discussions
/// view), this is what the **study agent** watches, so an `@`-mention posted anywhere — in a note's
/// comments as much as in a stand-alone discussion — is seen and answered. Same `{id, count}` shape.
pub fn thread_roots(store: &dyn Store) -> Result<Vec<ThreadRoot>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::NoteRef { key: crate::thread::THREAD_OF.into(), id: None }),
        ..Default::default()
    };
    let msgs = store.query(&q)?.rows;
    let mut count: std::collections::HashMap<Id, usize> = std::collections::HashMap::new();
    for o in &msgs {
        if let PropertyValue::Text(s) = o.get(crate::thread::THREAD_OF) {
            if let Some(root) = fm_model::parse_note_ref(&s) {
                // Register every root any message points at. A first-class discussion is self-anchored
                // (its own `thread_of` names itself) — that self-message is not a reply, so it just
                // registers the root at 0; replies (pointing at another note) add to that note's count.
                let e = count.entry(root).or_insert(0);
                if root != o.id {
                    *e += 1;
                }
            }
        }
    }
    Ok(count.into_iter().map(|(id, count)| ThreadRoot { id: id.to_string(), count }).collect())
}

/// Who has posted in each discussion, from **one vault's git log** — `root id → participants`,
/// newest-first, deduped by email. The dispatcher calls this per vault and merges the results into
/// `DiscussionSummary::participants`.
///
/// Split out from the dispatcher so the grouping is testable with real git. A discussion root and
/// its replies are all messages, and [`activity`] deliberately drops messages, so their authorship
/// cannot be read from there — it is gathered here. `since` is a git `--since` value; a wide
/// window is used in practice, because the roots are already listed by [`discussions`] and this only
/// adds authorship.
pub fn discussion_participants(
    store: &dyn Store,
    vault_path: &Path,
    since: &str,
) -> Result<std::collections::HashMap<String, Vec<Participant>>, StoreError> {
    let mut who: std::collections::HashMap<String, Vec<Participant>> = std::collections::HashMap::new();
    for t in fm_core::vcs::activity(vault_path, since)? {
        let Ok(id) = t.id.parse::<Id>() else { continue };
        let Some(obj) = store.get(id)? else { continue };
        // A reply names its root; a discussion root names itself. Anything else — an ordinary note,
        // or a comment on an ordinary note — is not a first-class discussion and is skipped.
        let root = match obj.get(crate::thread::THREAD_OF) {
            PropertyValue::Text(s) => match fm_model::parse_note_ref(&s) {
                Some(r) => r.to_string(),
                None => continue,
            },
            _ => continue,
        };
        let list = who.entry(root).or_default();
        if !list.iter().any(|p| p.email == t.email) {
            list.push(Participant { name: t.author, email: t.email });
        }
    }
    Ok(who)
}

/// Replace a note's body and write it back to disk (bumping `updated`). The
/// body is stored byte-for-byte — the editor is a plain textarea holding literal
/// Markdown, so the round-trip (edit -> store -> read) is lossless by
/// construction, the invariant the whole files-as-truth design rests on.
///
/// `base` is the **version** the caller last saw — the hash of the body it loaded — and it is
/// the lost-update guard for an editor that has been open a while. Returns the new version,
/// which the caller holds for its next write.
///
/// It was the `updated` stamp until 2026-07-18, which only caught writers that bump it. The
/// app does and the merge driver does; **Vim does not** — so a note hand-edited outside the
/// app could be overwritten by a stale editor, with `put`'s mtime guard already disarmed by
/// the poll's reindex. Content cannot lie about whether the body moved.
///
/// `FileStore::put` already refuses a write whose file moved on disk since we indexed it —
/// but that check cannot see this case. `pull` merges and then *reindexes*, because a merge
/// is invisible until it does; the reindex records the post-merge mtime, so from `put`'s
/// point of view everything is in sync while the open pane still holds pre-merge text. The
/// staleness is in the client, so the client has to be the one to declare what it edited.
/// Without this, a debounced auto-save silently overwrites a collaborator's merged
/// paragraph and leaves a clean history saying you wrote it.
///
/// An **empty `base` opts out** — `fm-cli`, curl, and anything that never read the note
/// keep working, still covered by the mtime guard in `put`.
pub fn update_body(
    store: &mut dyn Store,
    id: &str,
    body: &str,
    base: &str,
) -> Result<String, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    if !base.is_empty() && crate::dto::version_of(&obj.body) != base {
        return Err(StoreError::Conflict(id));
    }
    obj.body = body.to_string();
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(crate::dto::version_of(&obj.body))
}

/// Delete a note: remove its Markdown file and drop it from the index. The
/// destructive counterpart of `capture` — `Store::delete` already unlinks the
/// `.md` and both index rows, and returns `NotFound` for an unknown id, so this
/// is a thin, id-parsing wrapper (the UI gates it behind a second confirmation).
pub fn delete(store: &mut dyn Store, id: &str) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    store.delete(id)
}

/// Set one property and write it back to disk (bumping `updated`). This is the
/// board's drag write-back: dropping a card into a column calls this with the
/// column's `value`, so a drop and `fm set` change the file identically. An
/// empty `value` clears the property (the "(none)" column).
pub fn set_property(
    store: &mut dyn Store,
    id: &str,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    apply_property(&mut obj, key, value)?;
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(())
}

/// Full-text search across every note, newest-updated first. This is "zero new
/// code" again — the same engine and `ObjectMeta`, just a `Text` predicate. In
/// `FileStore` that predicate is answered by SQLite FTS5 (prefix terms, ranked);
/// in `MemoryStore` by a substring scan — so the one command works identically
/// against both, exactly as the CLI's `fm search` does. An empty needle returns
/// nothing rather than dumping the whole vault.
pub fn search(store: &dyn Store, query: &str) -> Result<Vec<ObjectMeta>, StoreError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let q = Query {
        filter: Filter::new().and(Predicate::Text(query.to_string())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Every note, newest-created first — the timeline/journal feed, and the
/// suggestion list behind a bare `/` in the editor. Assets are excluded (see
/// [`board`]); the renderer groups the rest by creation day into a Logseq-style
/// journal.
pub fn recent(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: crate::thread::notes_base(),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Every open proposal in one vault — the notes carrying a well-formed `proposes: branch:<name>`,
/// newest first. The Collaboration surface's feed, and the exact complement of the proposal
/// exclusion in [`crate::thread::notes_base`]: proposals are kept out of the planning views
/// precisely so they can be gathered here instead.
///
/// **Read-only, and derived.** This lists the proposal *notes* that exist; whether each branch is
/// still open, merged, or gone is a git question answered elsewhere (Ruling 15) — a note naming a
/// branch that no longer resolves is shown, not hidden, because the discussion outlives the branch.
///
/// Same `candidates()` seam and same O(corpus) cost as [`recent`] — a `BranchRef` predicate is
/// evaluated in memory over `load_all()`, never pushed to SQL. Proposals are a rare handful, so
/// this is deliberately left un-indexed; do not add one.
pub fn proposals(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: crate::thread::proposals_base(),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Create a **proposal** to change an existing note: the change lands on a fresh `proposal/<id>`
/// branch (never `main`), and a proposal note carrying `proposes: branch:<name>` records it for the
/// Collaboration view. The by-hand caller and (later) the study agent both come through here, so the
/// blast-radius bound is enforced in exactly **one** place.
///
/// **Guardrailed and fail-closed** ([`fm_core::proposal::ProposalLimits`]): the change is one file
/// of a known size, checked against the vault's per-proposal *and* per-vault ceilings, and a breach
/// is **refused, never truncated**. The branch is built first — additive, no `main` write — and the
/// proposal note is recorded only if that succeeds, so a refused or failed proposal leaves nothing
/// dangling. `vault_path` is the target's own vault (the caller resolves it); `limits` come from that
/// vault's `vault.json`.
pub fn create_proposal(
    store: &mut dyn Store,
    vault_path: &Path,
    target: &str,
    new_body: &str,
    limits: &fm_core::proposal::ProposalLimits,
    author: Option<(&str, &str)>,
) -> Result<ObjectMeta, StoreError> {
    let id: Id = target.parse().map_err(|_| StoreError::Parse(format!("invalid id: {target}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    if obj.kind != Kind::Note {
        return Err(StoreError::Io("you can only propose a change to a note".into()));
    }

    // The proposed file: the target note with its new body, serialized **exactly** as it would be
    // written to disk — the branch holds real note bytes (files-as-truth), not a diff.
    obj.body = new_body.to_string();
    let content =
        fm_core::frontmatter::to_file(&obj).map_err(|e| StoreError::Parse(e.to_string()))?;

    // The target note's repo-relative file path, from the vault's own notes directory.
    let desc = fm_core::descriptor::Descriptor::read(vault_path)?;
    let notes_abs = desc.notes_dir(vault_path);
    let rel = notes_abs
        .strip_prefix(vault_path)
        .unwrap_or(&notes_abs)
        .join(format!("{id}.md"))
        .to_string_lossy()
        .replace('\\', "/");

    // Commit the target note first (before any branch), so the proposal branch shares a base with
    // `main`. Without this an *uncommitted* note (a freshly captured one whose debounced auto-commit
    // hasn't fired) makes the branch ADD the file; accepting it then conflicts with main's untracked
    // copy — `main` stays safe, but the proposal can never merge. Idempotent + best-effort. `commit_all`
    // takes vault-rooted paths, so join `rel` onto the vault.
    let note_path = vault_path.join(&rel);
    let _ = fm_core::vcs::commit_all(
        vault_path,
        "auto: snapshot the note before a proposal",
        std::slice::from_ref(&note_path),
    );

    let title = obj.title.clone().unwrap_or_else(|| "note".into());

    // **One living proposal per note.** Find this note's existing OPEN proposal up front, so a REVISE is
    // guardrailed as a *replacement* (it does not add a proposal) rather than double-counting itself.
    let existing = open_proposal_for(store, vault_path, id)?;
    let existing_branch = existing.as_ref().and_then(proposal_branch_of);

    // Guardrails: one file of `content.len()` bytes, against the vault's ceilings and current load.
    // Refused **before** any write, fail-closed, never truncated. On a revise, subtract the proposal
    // being replaced from the load — it is rewritten, not added, so it must not count against its own
    // ceiling (otherwise a busy vault could never refine its one proposal).
    let (mut open, mut open_bytes) = fm_core::vcs::proposal_load(vault_path)?;
    if let Some(branch) = &existing_branch {
        open = open.saturating_sub(1);
        if let Some(cur) = fm_core::vcs::file_on_branch(vault_path, branch, &rel) {
            open_bytes = open_bytes.saturating_sub(cur.len() as u64);
        }
    }
    limits
        .check(
            fm_core::proposal::ProposalSize { files: 1, bytes: content.len() as u64 },
            fm_core::proposal::VaultLoad { open, open_bytes },
        )
        .map_err(|b| StoreError::Io(b.to_string()))?;

    // REVISE the note's existing proposal in place — an ATOMIC ref move (never delete first, so a failed
    // or interrupted build leaves the prior revision exactly as it was) — or open a new one. The
    // proposal note is unchanged; the discussion stays on the *original* note.
    if let (Some(existing), Some(branch)) = (existing, existing_branch) {
        fm_core::vcs::revise_proposal_branch(
            vault_path,
            &branch,
            &rel,
            &content,
            &format!("revise: change to {title}"),
            author,
        )?;
        if fm_core::vcs::remote(vault_path)?.is_some() {
            let _ = fm_core::vcs::push_branch(vault_path, &branch, true); // the ref moved → force-push
        }
        return Ok(ObjectMeta::from(&existing));
    }

    // No open proposal yet — open a new one. Its own id names the branch; `targets: note:<id>` (a
    // reference value, like `thread_of`) ties it back to the note so the next proposal finds and
    // refines it.
    let mut note = Object::new(Kind::Note, format!("Proposed change to **{title}**."));
    note.vault = obj.vault.clone();
    note.title = Some(format!("Proposal: {title}"));
    let branch = format!("proposal/{}", note.id);
    note.extra
        .insert(crate::thread::PROPOSES.into(), PropertyValue::Text(fm_model::branch_ref(&branch)));
    note.extra.insert(crate::thread::TARGETS.into(), PropertyValue::Text(fm_model::note_ref(id)));

    // Build the branch first (additive, no `main` write); record the note only on success.
    fm_core::vcs::create_proposal_branch(
        vault_path,
        &branch,
        &rel,
        &content,
        &format!("propose: change to {title}"),
        author,
    )?;
    // Share the branch so *another user* can review and accept it — a push of `main` never carries
    // `proposal/*`. Best-effort: a vault with no remote (or an offline push) still has a valid local
    // proposal; it just stays single-user until the branch reaches the remote.
    if fm_core::vcs::remote(vault_path)?.is_some() {
        let _ = fm_core::vcs::push_branch(vault_path, &branch, false);
    }
    store.put(&note)?;
    Ok(ObjectMeta::from(&note))
}

/// The note's current OPEN proposal, if any: a proposal note that `targets` `host` and whose branch
/// still resolves. This is what makes a note's PR a single living thing — the next `/research` or
/// `/propose` refines it rather than stacking a new one, and a merged/rejected proposal (branch gone)
/// is skipped so a fresh proposal opens cleanly.
fn open_proposal_for(
    store: &dyn Store,
    vault_path: &Path,
    host: Id,
) -> Result<Option<Object>, StoreError> {
    for meta in proposals(store)? {
        let Ok(pid) = meta.id.parse::<Id>() else { continue };
        let Some(p) = store.get(pid)? else { continue };
        // Cheap in-memory filters BEFORE the git spawn: only THIS note's proposal, and never a declined
        // (closed) record — so `branch_open` runs at most for this note's live candidate.
        let targets = match p.get(crate::thread::TARGETS) {
            PropertyValue::Text(s) => fm_model::parse_note_ref(&s),
            _ => None,
        };
        if targets != Some(host) || is_declined(&p) {
            continue;
        }
        if let Some(branch) = proposal_branch_of(&p) {
            if fm_core::vcs::branch_open(vault_path, &branch) {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}

/// The branch a proposal note names via `proposes: branch:<name>`, if well-formed.
fn proposal_branch_of(obj: &Object) -> Option<String> {
    match obj.get(crate::thread::PROPOSES) {
        PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).map(String::from),
        _ => None,
    }
}

/// Whether a proposal note is a rejected (declined) record. Stored as a bool so a hand-edited
/// `declined: true` in the note's frontmatter reads correctly (a bool-shaped *string* would not); the
/// string form is still tolerated for any older marker.
fn is_declined(obj: &Object) -> bool {
    match obj.get(crate::thread::DECLINED) {
        PropertyValue::Bool(b) => b,
        PropertyValue::Text(s) => s == "true",
        _ => false,
    }
}

/// The id of the note's current OPEN proposal, or `None`. Lets the note's *own* view surface its PR —
/// the diff and Accept/Reject — beside the discussion, since a proposal has no separate discussion of
/// its own: the conversation and the PR live together on the originating note.
pub fn proposal_for(
    store: &dyn Store,
    vault_path: &Path,
    host: &str,
) -> Result<Option<String>, StoreError> {
    let Ok(hid) = host.parse::<Id>() else { return Ok(None) };
    Ok(open_proposal_for(store, vault_path, hid)?.map(|p| p.id.to_string()))
}

/// A proposal's change, for review: whether its branch still resolves, the files it touches, and the
/// unified diff. `exists: false` (with an empty diff) is the normal, non-error answer for a proposal
/// whose branch has been merged or deleted — the note outlives the branch.
#[derive(Serialize)]
pub struct ProposalDiff {
    pub exists: bool,
    /// This proposal was **rejected** (branch gone, note kept as a record). Distinguishes a declined PR
    /// from a merged one — both have `exists: false` — so the review UI can label it correctly.
    pub declined: bool,
    pub files: Vec<String>,
    pub patch: String,
}

/// Read the diff of the proposal note `id` — resolve its `proposes: branch:<name>` and diff that
/// branch against `main`. Refuses a note that is not a proposal (no well-formed `proposes:`), the
/// read-side twin of [`create_proposal`]. `vault_path` is the proposal's own vault.
pub fn proposal_diff(
    store: &dyn Store,
    vault_path: &Path,
    id: &str,
) -> Result<ProposalDiff, StoreError> {
    let pid: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    // A gone proposal *note* (just rejected, or a stale Collaboration list) is "nothing to show", not
    // an error — the same tolerance a gone *branch* already gets below (the note outlives the branch;
    // here the note itself is gone). This is what a client sees after Reject before its list refreshes.
    let Some(obj) = store.get(pid)? else {
        return Ok(ProposalDiff { exists: false, declined: false, files: Vec::new(), patch: String::new() });
    };
    let declined = is_declined(&obj);
    let branch = match obj.get(crate::thread::PROPOSES) {
        PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).map(String::from),
        _ => None,
    }
    .ok_or_else(|| StoreError::Io(format!("{id} is not a proposal")))?;

    let (exists, files, patch) = fm_core::vcs::branch_diff(vault_path, &branch)?;
    Ok(ProposalDiff { exists, declined, files, patch })
}

/// The PROPOSED note behind a proposal, for review: the host note it edits, its title, and the
/// proposed **body** as it stands on the branch.
#[derive(Serialize)]
pub struct ProposalContent {
    /// The note this proposal edits — so the review can save an edit back through `create_proposal`.
    pub host: String,
    pub title: String,
    /// The proposed note body (from the branch) — the review VISUALIZES this and lets the user EDIT it.
    pub body: String,
}

/// Read a proposal's proposed note — host id, title, and the proposed body on the branch. Lets the
/// review UI show the proposed note and **edit it before accepting**: saving an edit goes back through
/// [`create_proposal`], which rebuilds the branch from the current `main`, so editing also **resolves a
/// stale conflict**. `None` when the branch is gone (merged/declined) — nothing to edit.
pub fn proposal_content(
    store: &dyn Store,
    vault_path: &Path,
    id: &str,
) -> Result<Option<ProposalContent>, StoreError> {
    let pid: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let Some(obj) = store.get(pid)? else { return Ok(None) };
    let branch = match obj.get(crate::thread::PROPOSES) {
        PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).map(String::from),
        _ => None,
    };
    let Some(branch) = branch else { return Ok(None) };
    let (exists, files, _) = fm_core::vcs::branch_diff(vault_path, &branch)?;
    if !exists {
        return Ok(None);
    }
    let Some(rel) = files.first() else { return Ok(None) };
    let Some(file) = fm_core::vcs::file_on_branch(vault_path, &branch, rel) else { return Ok(None) };
    let proposed = fm_core::frontmatter::from_file(&file).map_err(|e| StoreError::Parse(e.to_string()))?;
    // The host note id is the proposed file's stem (works whatever the vault's notes dir), and the
    // title comes from the proposed note, falling back to the live host note's.
    let host =
        std::path::Path::new(rel).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let title = proposed
        .title
        .clone()
        .or_else(|| host.parse::<Id>().ok().and_then(|h| store.get(h).ok().flatten()).and_then(|o| o.title))
        .unwrap_or_else(|| "note".into());
    Ok(Some(ProposalContent { host, title, body: proposed.body }))
}

/// Accept the proposal note `id`: resolve its `proposes: branch:<name>` (exactly as [`proposal_diff`]
/// does) and merge that branch into the current branch. The write half of review — the GUI's "Accept"
/// button. Fail-closed: an unclean merge is aborted and `main` is left untouched
/// ([`fm_core::git::Accepted::Conflicted`]). Refuses a note that is not a proposal.
pub fn accept_proposal(
    store: &dyn Store,
    vault_path: &Path,
    id: &str,
) -> Result<fm_core::git::Accepted, StoreError> {
    let pid: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let obj = store.get(pid)?.ok_or(StoreError::NotFound(pid))?;
    let branch = match obj.get(crate::thread::PROPOSES) {
        PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).map(String::from),
        _ => None,
    }
    .ok_or_else(|| StoreError::Io(format!("{id} is not a proposal")))?;

    fm_core::vcs::merge_proposal_branch(vault_path, &branch)
}

/// Reject the proposal note `id`: delete its branch (local + remote if it was pushed) and the proposal
/// note itself, so it leaves the Collaboration view. The GUI's "Reject" button — the safe inverse of
/// accept. **`main` is never touched**: a proposal is only ever an off-`main` branch, so declining it
/// is just deleting that branch, and the worst case of a mistaken reject is a proposal you re-run.
/// Refuses a note that is not a proposal.
pub fn reject_proposal(store: &mut dyn Store, vault_path: &Path, id: &str) -> Result<(), StoreError> {
    let pid: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(pid)?.ok_or(StoreError::NotFound(pid))?;
    let branch = match obj.get(crate::thread::PROPOSES) {
        PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).map(String::from),
        _ => None,
    }
    .ok_or_else(|| StoreError::Io(format!("{id} is not a proposal")))?;

    // Delete the branch (the change is dropped; `main` is never touched), but KEEP the proposal note as
    // a record of a declined PR — a proposal note is immortal, exactly as a merged one's is. Mark it
    // `declined` so it reads as rejected (not merged), and so a fresh `/research` on the note opens a new
    // PR rather than mistaking this closed one for the live proposal (its branch is gone anyway).
    fm_core::vcs::delete_branch(vault_path, &branch)?;
    obj.extra.insert(crate::thread::DECLINED.into(), PropertyValue::Bool(true));
    store.put(&obj)?;
    Ok(())
}

/// Notes that link **to** `id` — the reverse of the `note:` references a body makes. "What points
/// here", the backlinks panel's feed, newest-updated first.
///
/// **Derived by scanning, never indexed.** `refs::references` extracts each note's outbound `note:`
/// refs (the same primitive `copy_note` uses); a note is a backlink when `id` is among them — a
/// plain `note:` mention or an `![](note:id)` embed both count. Same `candidates()`/`load_all()`
/// O(corpus) cost as `recent`/`thread`, already accepted at 10k scale, so there is **no reverse
/// index to keep true** — the files stay the one truth. The target itself is never its own backlink.
pub fn backlinks(store: &dyn Store, id: &str) -> Result<Vec<ObjectMeta>, StoreError> {
    let target: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let q = Query {
        filter: crate::thread::notes_base(),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| o.id != target)
        .filter(|o| {
            let (_assets, notes) = refs::references(&o.body);
            notes.iter().any(|n| n.parse::<Id>().ok() == Some(target))
        })
        .map(ObjectMeta::from)
        .collect())
}

/// True when a note body still carries git conflict markers.
///
/// **Delegates to `fm_core::merge`, deliberately.** The same predicate now also guards
/// `vcs::resolve_conflict(_, _, Keep::Edited)` — "I reconciled this in the editor" — and the two must
/// never disagree: a list that calls a note clean while the guard refuses it (or the reverse) is how
/// `<<<<<<<` ends up committed as a note's content and pushed to a collaborator. One definition.
fn has_conflict_markers(body: &str) -> bool {
    fm_core::merge::has_conflict_markers(body)
}

/// Notes that came back from a merge **in conflict** — both versions are marked in the body and a
/// human must settle them. The Collaboration surface lists these beside proposals: both are "needs a
/// person", and a conflict left unresolved blocks every commit, so it must be findable, not just a
/// toast that scrolled away.
///
/// **Derived by scanning, no stored state** (like `recent`/`backlinks`): a note is in conflict iff
/// its body holds the markers, so the list is always exactly the current truth on disk — resolve a
/// note and it drops off by itself. Same O(corpus) `load_all()` cost, accepted at 10k scale.
pub fn conflicts(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: crate::thread::notes_base(),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| has_conflict_markers(&o.body))
        .map(ObjectMeta::from)
        .collect())
}


/// **Notes that are byte-for-byte the same note, written more than once.**
///
/// Grouped by the **body**, because copies differ in `id` and `created` — that is what makes them
/// copies rather than one note. Keyed the same way `dto::UnrecordedNote::copies` counts them, so the
/// two surfaces cannot disagree about what a duplicate is.
///
/// Found on the owner's phone 2026-07-31: one prompt to the study assistant written ~142 times inside
/// one minute, at machine pace. A scan and not stored state (like `conflicts`/`recent`): a family that
/// is pruned drops off by itself, and there is no bookkeeping to go stale.
///
/// **The oldest copy is the keeper**, by `created` then `id` — deterministic, and the one that is most
/// likely to be the note the user actually made before whatever loop copied it.
pub fn duplicates(store: &dyn Store) -> Result<Vec<DuplicateFamily>, StoreError> {
    // **Every note, not `notes_base()`.** That filter excludes discussion messages and proposals
    // because they are not things you *plan* — right for a board, wrong here: the duplicates that
    // prompted this were **messages**, so scanning with it found nothing at all and the surface stayed
    // empty while 142 copies sat on disk. Assets are excluded because a duplicate blob is the blob
    // store's business (content-addressing already dedupes bytes) and an asset note is a catalogue
    // entry, not a copy of anything.
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![fm_model::Kind::Note])),
        ..Default::default()
    };
    let mut by_body: std::collections::HashMap<String, Vec<Object>> =
        std::collections::HashMap::new();
    for o in store.query(&q)?.rows {
        let body = o.body.trim();
        if body.is_empty() {
            continue; // an empty note is not a copy of another empty note in any useful sense
        }
        by_body.entry(fm_core::blob::sha256_hex(body.as_bytes())).or_default().push(o);
    }
    let mut out: Vec<DuplicateFamily> = by_body
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(hash, mut v)| {
            v.sort_by(|a, b| a.created.cmp(&b.created).then_with(|| a.id.cmp(&b.id)));
            let keep = v.remove(0);
            DuplicateFamily {
                body: hash,
                vault: keep.vault.clone(),
                preview: keep
                    .body
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .map(|l| l.chars().take(72).collect())
                    .unwrap_or_default(),
                keep: keep.id.to_string(),
                extras: v.iter().map(|o| o.id.to_string()).collect(),
            }
        })
        .collect();
    // Biggest family first: that is the one that names the loop.
    out.sort_by(|a, b| b.extras.len().cmp(&a.extras.len()));
    Ok(out)
}

/// One family of identical notes: which copy is kept, and which are extras.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DuplicateFamily {
    /// sha256 of the shared body — the identity of the family, stable across pruning.
    pub body: String,
    pub vault: String,
    /// The first line of the shared body, so a human can recognise what was duplicated.
    pub preview: String,
    /// The copy that stays: oldest by `created`, then by id. **Never offered for deletion.**
    pub keep: String,
    /// The copies that may be removed. Always at least one, or this is not a family.
    pub extras: Vec<String>,
}

/// The tag that marks a note as a template — a starting point to spin new notes from.
pub const TEMPLATE_TAG: &str = "template";

/// Every note tagged [`TEMPLATE_TAG`] — the "New from template" list, most-recently-touched first.
///
/// **A template is just a tagged note**, nothing more: no new `Kind`, no reserved property, no
/// hidden note-class. Tag any note `template` (in the editor's Tags field) and it becomes a
/// starting point; untag it and it stops being one. Because it stays an ordinary note it still
/// shows on the board/timeline and is fully searchable — the honest cost of not inventing a type.
///
/// Same in-memory `TagsAll` filter and O(corpus) `load_all()` cost as [`recent`]/[`backlinks`] —
/// templates are a rare handful, so there is deliberately no index.
pub fn templates(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: crate::thread::notes_base().and(Predicate::TagsAll(vec![TEMPLATE_TAG.into()])),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Normalize an asset reference to its blob hash. Notes, the gallery, and the
/// mock all spell the same blob differently — stored as `sha256:<hex>`, written
/// in Markdown as `asset:sha256-<hex>`, or passed bare — so every asset path
/// funnels through here to one lowercase hex hash.
fn parse_ref(reference: &str) -> Result<String, StoreError> {
    let r = reference.trim();
    let r = r.strip_prefix("asset:").unwrap_or(r);
    let r = r.strip_prefix("sha256:").or_else(|| r.strip_prefix("sha256-")).unwrap_or(r);
    // **A fragment is a viewer's business, never the blob's.** `#page=4` is the standard PDF open
    // parameter — a real reader honours it, which is what makes an anchored reference degrade to a
    // working link outside this app. It says nothing about *which* bytes are wanted, so it is cut
    // here rather than failing the all-hex check below: without this, every anchored reference is
    // `not an asset reference`, and `asset_status`/`resolve_asset`/`GET /api/blob` all route
    // through this one function.
    let r = r.split('#').next().unwrap_or(r);
    let hash = r.trim();
    if hash.len() < 4 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(StoreError::Parse(format!("not an asset reference: {reference}")));
    }
    Ok(hash.to_ascii_lowercase())
}

/// Read the bytes of a referenced asset. `kind` selects the derived thumbnail
/// (`"thumb"`) or the full blob (anything else). A missing blob is an ordinary `Err`,
/// which the UI degrades to the "asset not available" placeholder (media absence is a
/// warning, never a crash).
///
/// **Not how inline media reaches the read view any more.** This reads the whole file into
/// memory to hand it back, which is the wrong shape for a 300 MB video and cannot seek;
/// `<img>`/`<video>`/`<iframe>` point at the streaming `GET /api/blob/<reference>` route
/// instead (`fm-serve/src/blob.rs`). What still needs this: thumbnails, and any frontend
/// with no HTTP route to stream from — which is why it stays.
pub fn resolve_asset_bytes(vault: &Path, reference: &str, kind: &str) -> Result<Vec<u8>, StoreError> {
    let hash = parse_ref(reference)?;
    let path = match kind {
        "thumb" => ingest::thumb_path(vault, &hash),
        _ => BlobStore::new(vault).path_for(&hash),
    };
    std::fs::read(&path).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
}

/// Whether a referenced asset can be shown: is the blob present locally, and has
/// a thumbnail been generated? The UI uses this to choose between a real preview,
/// an "open externally" affordance, and the missing-asset placeholder.
#[derive(Clone, Debug, Serialize)]
pub struct AssetStatus {
    pub has_blob: bool,
    pub has_thumb: bool,
    /// Sniffed MIME of the blob (magic bytes) — the read view picks its inline
    /// element from this. `None` when the blob is absent or has no signature.
    pub mime: Option<String>,
}

pub fn asset_status(vault: &Path, reference: &str) -> Result<AssetStatus, StoreError> {
    let hash = parse_ref(reference)?;
    let store = BlobStore::new(vault);
    // **A zero-byte blob is not media.** Ingest refuses an empty body now, so the only way one
    // exists is as debris from before that guard: on Android every capture was stored as zero
    // bytes, all colliding on the empty string's hash. Those notes would otherwise report a blob
    // that is present, hand the UI a URL, and render as a broken image icon — which says nothing.
    // Treated as absent, the note shows the ordinary "no bytes in this vault" placeholder, which
    // is exactly what happened.
    let has_blob = store.exists(&hash)
        && std::fs::metadata(store.path_for(&hash)).map(|m| m.len() > 0).unwrap_or(false);
    Ok(AssetStatus {
        has_blob,
        has_thumb: ingest::thumb_path(vault, &hash).exists(),
        mime: has_blob.then(|| ingest::sniff_mime(&store.path_for(&hash))).flatten(),
    })
}

/// What is actually at a path, for the "create a vault here?" form to answer with.
///
/// Facts only — every one of these is *reported*, and what is refused versus merely warned
/// about is policy, decided by the caller that also knows the vault list. Splitting it here
/// is the same seam [`asset_status`] uses: this crate knows the filesystem, not the config.
#[derive(Clone, Debug, Serialize)]
pub struct PathFacts {
    /// Absolute; the caller expands `~` before handing it over.
    pub path: String,
    pub exists: bool,
    /// No entries at all. Not an error — see `notes`.
    pub empty: bool,
    /// `.md` files directly under `<path>/notes`. **Adoption is free** — `FileStore::named`
    /// reindexes whatever is already there, so this needs no code. But it must be *said*:
    /// creating a vault over someone's notes silently is the surprise this field prevents.
    pub notes: usize,
    pub not_a_directory: bool,
    /// The parent doesn't exist either, so we'd be creating a chain of directories.
    pub parent_missing: bool,
    /// **Probed, not inferred from mode bits.** We create and remove a temp entry, because
    /// a capability must mean "this will work", never "this is configured" — which is
    /// exactly how `restic_ready` came to enable a checkbox that then failed. Root, and a
    /// read-only mount that lies about its permissions, are why the bits are not the answer.
    pub writable: bool,
    /// Already a git repo. Not an error: we leave its history alone, and its identity is
    /// one fewer thing to ask the user for.
    pub git_repo: bool,
}

/// Inspect a path for the create-vault form. Never mutates anything that outlives the
/// call: the write probe cleans up after itself.
pub fn inspect_path(path: &Path) -> PathFacts {
    let md = std::fs::metadata(path);
    let exists = md.is_ok();
    let not_a_directory = md.as_ref().map(|m| !m.is_dir()).unwrap_or(false);
    let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);

    let empty = is_dir
        && std::fs::read_dir(path).map(|mut d| d.next().is_none()).unwrap_or(false);

    // Only the notes directory's `*.md`, non-recursively — `FileStore::reindex` reads exactly
    // that, so counting anything else here would promise notes that never appear.
    //
    // **Which directory that is comes from the folder's own `vault.json`**, not from the
    // literal `notes`. A folder being adopted may already be a formicaria vault (that is what
    // `clone_vault` and `restore_vault` hand this), and one that puts its notes in `docs/`
    // would otherwise be previewed as "0 notes" right before the app opened it and found
    // hundreds — the preview contradicting the thing it is previewing.
    let notes_dir = fm_core::descriptor::Descriptor::read(path)
        .map(|d| d.notes_dir(path))
        .unwrap_or_else(|_| path.join("notes"));
    let notes = std::fs::read_dir(notes_dir)
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0);

    // The nearest existing ancestor is what we can actually probe: the path itself may
    // not exist yet, and "can I create it?" is a question about its parent.
    let probe_at = if is_dir { Some(path.to_path_buf()) } else { nearest_existing(path) };
    let parent_missing = !exists && probe_at.as_deref() != path.parent();

    PathFacts {
        path: path.to_string_lossy().into_owned(),
        exists,
        empty,
        notes,
        not_a_directory,
        parent_missing,
        writable: probe_at.map(|p| can_write(&p)).unwrap_or(false),
        git_repo: path.join(".git").exists(),
    }
}

/// The closest ancestor that exists, so a path we are about to create can still be
/// probed. `None` when even the root is unreachable.
fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut p = path.parent()?;
    loop {
        if p.is_dir() {
            return Some(p.to_path_buf());
        }
        p = p.parent()?;
    }
}

/// Can we really write here? Make something and remove it. Mode bits are configuration;
/// this is capability.
fn can_write(dir: &Path) -> bool {
    let probe = dir.join(format!(".fm-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Ingest an uploaded file: store its bytes as a content-addressed blob, extract
/// searchable text, and create an asset note pointing at it — the GUI twin of
/// `fm add`. Returns the new asset's meta so the editor can insert a reference
/// (`![title](asset:sha256-<hash>)`) without a refetch.
/// `vault` is where the bytes go; `vault_name` is the audience the asset **note** joins.
/// Both, because they are answered by different things: the blob store takes a path, and
/// the `Store` routes by name. They must agree — an asset note in one vault describing a
/// blob in another means the people who can see the file cannot see the note, and the
/// person who can see the note is pointing at bytes they never shared.
pub fn ingest(
    store: &mut dyn Store,
    vault: &Path,
    vault_name: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<ObjectMeta, StoreError> {
    let ing = ingest::ingest_bytes(vault, filename, bytes)?;
    asset_note(store, vault, vault_name, &ing)
}

/// Turn an ingested blob into its asset note, and store it.
///
/// **Shared on purpose.** This is the shape of an asset note — filename as the title, the
/// blob hash in `assets`, the MIME as a queryable property, and the extracted text as the
/// body so it is full-text searchable and travels with the notes rather than the blob. It was
/// written twice, here and in `fm add`, and the copies had already diverged: the CLI's never
/// set `obj.vault`, so a file added from the command line was stamped with no audience.
///
/// The two callers differ only in how the bytes arrive — `ingest` has them in memory (an
/// upload), `fm add` streams them from a path so a large file is never held whole. That
/// difference is real and worth keeping; the note is not.
pub fn asset_note(
    store: &mut dyn Store,
    vault: &Path,
    vault_name: &str,
    ing: &fm_core::Ingested,
) -> Result<ObjectMeta, StoreError> {
    let mut obj = Object::new(Kind::Asset, ing.text.clone().unwrap_or_default());
    obj.title = Some(ing.filename.clone());
    obj.assets = vec![format!("sha256:{}", ing.hash)];
    obj.extra.insert("mime".into(), PropertyValue::Text(ing.mime.clone()));
    // **The identifier a paper prints on itself, for free.** `pdftotext` has already run, and most
    // modern papers put their DOI or arXiv id on page one — so the commonest way a paper enters
    // the vault needs no typing and no network. Only the *front* of the text is scanned: a DOI in
    // the bibliography belongs to somebody else's paper (see `paper::identifier_in_text`).
    //
    // Recorded on the asset note, which is what knows about this file. Attaching it to a paper
    // note is the caller's business — this layer does not decide what the PDF is *of*.
    if let Some(text) = ing.text.as_deref() {
        if let Some(id) = crate::paper::identifier_in_text(text) {
            obj.extra
                .insert(id.key().to_string(), PropertyValue::Text(id.value().to_string()));
        }
    }
    obj.vault = vault_name.to_string();
    store.put(&obj)?;
    // Best-effort thumbnail: a missing vipsthumbnail (or a failure) only degrades a gallery
    // tile, never the ingest.
    let _ = ingest::thumbnail(vault, &ing.hash);
    Ok(ObjectMeta::from(&obj))
}

/// A recent note edit, as the collaboration views show it: git says who last touched a note and
/// when (`fm_core::git::Touch`); the store says what that note currently is. One `git log` per
/// vault powers the authorship labels, the activity stream, and the contributor filter.
#[derive(Serialize)]
pub struct EditEvent {
    pub id: String,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub vault: String,
    pub author: String,
    pub email: String,
    pub time: String,
}

/// Recent edits in one vault, read from git and resolved against the store. A touch whose note is
/// gone from the index (deleted, or not yet reindexed) is dropped — the view shows notes that
/// exist. `since` is a git `--since` value (e.g. `"1 year ago"`). Read-only.
pub fn activity(
    store: &dyn Store,
    vault_path: &Path,
    since: &str,
) -> Result<Vec<EditEvent>, StoreError> {
    let mut events = Vec::new();
    for t in fm_core::vcs::activity(vault_path, since)? {
        let Ok(id) = t.id.parse::<Id>() else { continue };
        let Some(obj) = store.get(id)? else { continue };
        // A `git log` read-model has no `Filter` to hang the exclusion on, so the hidden
        // note-classes are applied here by hand — the same list `notes_base()` excludes, on the
        // one surface a predicate cannot reach. Every reply and every proposal is a new file and
        // therefore a git touch: without this a busy thread (or a pile of proposals) floods the
        // recent-edits feed, the contributor filter and the `EditedBy` labels.
        if crate::thread::is_message(&obj) || crate::thread::is_proposal(&obj) {
            continue;
        }
        events.push(EditEvent {
            id: t.id,
            title: obj.title.clone(),
            kind: obj.kind.as_str().to_string(),
            vault: obj.vault.clone(),
            author: t.author,
            email: t.email,
            time: t.time,
        });
    }
    Ok(events)
}

/// The outcome of a copy: the new note's meta, the blob hashes this copy actually wrote
/// into the target (deduped ones are omitted, so an Undo knows exactly what to take back),
/// and how many prior copies of the same source it replaced.
#[derive(Serialize)]
pub struct CopyResult {
    pub meta: ObjectMeta,
    pub new_blobs: Vec<String>,
    pub replaced: usize,
}

/// A stable, one-way fingerprint of a source note's id, stamped on its copies as `copy_of`.
/// Re-copying the same source into a vault finds its prior copy by this and replaces it, so a
/// vault never accumulates duplicate copies — and the fingerprint reveals neither the id nor
/// the origin vault (unlike embedding the raw `note:<id>`, which is exactly what we strip).
fn provenance(source_id: Id) -> String {
    fm_core::blob::sha256_hex(source_id.to_string().as_bytes())
}

/// Does `target_vault` already hold a copy of the note `id`? The pre-check behind the
/// "this will replace the existing copy" warning.
pub fn copy_status(store: &dyn Store, id: &str, target_vault: &str) -> Result<bool, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let token = provenance(id);
    let want = PropertyValue::Text(token);
    Ok(store
        .candidates(&Filter::new())?
        .0
        .iter()
        .any(|o| o.vault == target_vault && o.get("copy_of") == want))
}

/// Copy a note into another vault. **Restrictive by default:** only the prose travels —
/// every `note:`/`asset:` reference is stripped (see [`refs::strip_cross_vault`]) so the
/// copy can never point at a note or blob outside its new audience, and `assets`/`code`
/// are cleared. `with_assets` opts in to carrying the note's first-degree blobs *into* the
/// target so it is self-contained (note links are still stripped — the linked-notes tier is
/// deferred). A copy is a **new** note (fresh ULID); the source is untouched. `vault_paths`
/// is every vault's `(name, root)`, used to locate a blob wherever it lives and to write the
/// target. An unknown `target_vault`, or the note's own vault, is refused.
pub fn copy_note(
    store: &mut dyn Store,
    id: &str,
    target_vault: &str,
    vault_paths: &[(String, PathBuf)],
    with_assets: bool,
) -> Result<CopyResult, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let src = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    if src.vault == target_vault {
        return Err(StoreError::Io(format!("note is already in vault '{target_vault}'")));
    }
    let target_path = vault_paths
        .iter()
        .find(|(name, _)| name == target_vault)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| StoreError::Io(format!("no vault named '{target_vault}'")))?;

    // Override, don't duplicate: a re-copy of the same source replaces its prior copies in this
    // vault (found by the `copy_of` fingerprint), so the vault never grows two copies of one note.
    let token = provenance(id);
    let want = PropertyValue::Text(token.clone());
    let stale: Vec<Id> = store
        .candidates(&Filter::new())?
        .0
        .iter()
        .filter(|o| o.vault == target_vault && o.get("copy_of") == want)
        .map(|o| o.id)
        .collect();
    let replaced = stale.len();
    for old in stale {
        store.delete(old)?;
    }

    // The copy: a fresh identity in the target vault, prose rewritten to drop outward refs,
    // stamped with the source fingerprint so a future re-copy finds and replaces it.
    let now = OffsetDateTime::now_utc();
    let mut obj = src.clone();
    obj.id = Ulid::new();
    obj.created = now;
    obj.updated = now;
    obj.vault = target_vault.to_string();
    obj.body = refs::strip_cross_vault(&src.body, with_assets);
    // **Every piece of user-authored text, not just the body.** Stripping only the body let a
    // pointer through into the copy — and so into permanent git history — against
    // `decisions.md`'s "a copy can never point outside its new vault".
    //
    // The list below is the whole of `Object` that a human can type into. It is deliberately
    // written out field by field rather than looped, because the first version of this fix
    // handled `title` + `extra` and silently missed `status` and `tags` — they are *typed*
    // fields, so they never appeared in the `extra` map the fix was reasoning about, and
    // `frontmatter` parses hand-written `status:`/`tags:` straight into them. `status` is
    // free-form (`board` groups by any string) and a tag is any string. Both went across
    // verbatim. If a field is added to `Object` that holds user text, it belongs here.
    //
    // Not stripped, because they cannot carry a reference: `due`/`start` (typed stamps),
    // `hard` (bool), `created`/`updated` (our own timestamps), `id`/`vault` (reset above).
    obj.title = obj.title.map(|t| refs::strip_cross_vault(&t, with_assets));
    obj.status = obj.status.map(|s| refs::strip_cross_vault(&s, with_assets));
    obj.tags = obj.tags.iter().map(|t| refs::strip_cross_vault(t, with_assets)).collect();
    // Keys as well as values: a property key is as user-typed as its value, and
    // `note:01ARZ…: something` is a surviving pointer however silly it looks.
    obj.extra = obj
        .extra
        .into_iter()
        .map(|(k, v)| {
            (refs::strip_cross_vault(&k, with_assets), refs::strip_value(&v, with_assets))
        })
        .collect();
    obj.code.clear(); // code blobs are not carried in v1 — never leave an outward pointer
    obj.extra.insert("copy_of".to_string(), PropertyValue::Text(token));

    let mut new_blobs = Vec::new();
    if with_assets {
        // First-degree asset hashes: the body's refs plus any frontmatter `assets`.
        let (mut hashes, _notes) = refs::references(&src.body);
        for a in &src.assets {
            if let Ok(h) = parse_ref(a) {
                hashes.push(h);
            }
        }
        hashes.sort();
        hashes.dedup();
        let target_blobs = BlobStore::new(&target_path);
        for h in &hashes {
            // Find the blob wherever it physically lives, and copy it into the target
            // (content-addressed, so put_file dedups; we record only what was new).
            if let Some((_, src_root)) = vault_paths.iter().find(|(_, p)| BlobStore::new(p).exists(h)) {
                let src_blob = BlobStore::new(src_root).path_for(h);
                let stored = target_blobs.put_file(&src_blob)?;
                if !stored.deduped {
                    new_blobs.push(stored.hash);
                }
            }
        }
        if !new_blobs.is_empty() {
            Manifest::build(&target_path)?.write(&target_path)?;
        }
    } else {
        // Prose-only: no attachment reference of any kind leaves the source vault.
        obj.assets.clear();
    }

    store.put(&obj)?;
    Ok(CopyResult { meta: ObjectMeta::from(&obj), new_blobs, replaced })
}

/// Recede a copy: delete the copied note from its vault, then remove the blobs this copy
/// newly wrote — but only a blob that no note remaining in the target still references
/// (the bytes are content-addressed, so a survivor may now be legitimately shared). The
/// inverse of the `with_assets` branch of [`copy_note`], and safe because the copy has a
/// fresh id with no inbound links yet.
pub fn uncopy_note(
    store: &mut dyn Store,
    id: &str,
    target_vault: &str,
    blobs: &[String],
    vault_paths: &[(String, PathBuf)],
) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    store.delete(id)?;
    if blobs.is_empty() {
        return Ok(());
    }
    let target_path = vault_paths
        .iter()
        .find(|(name, _)| name == target_vault)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| StoreError::Io(format!("no vault named '{target_vault}'")))?;

    // After the delete, which of these hashes does a note *remaining* in the target vault
    // still reference (in frontmatter `assets` or body)?
    let remaining = store.candidates(&Filter::new())?.0;
    let still_used = |hash: &str| {
        remaining.iter().filter(|o| o.vault == target_vault).any(|o| {
            o.assets.iter().any(|a| parse_ref(a).map(|h| h == hash).unwrap_or(false))
                || refs::references(&o.body).0.iter().any(|h| h == hash)
        })
    };

    let target_blobs = BlobStore::new(&target_path);
    let mut changed = false;
    for h in blobs {
        if !still_used(h) {
            let p = target_blobs.path_for(h);
            if p.exists() {
                std::fs::remove_file(&p).map_err(|e| StoreError::Io(e.to_string()))?;
                changed = true;
            }
        }
    }
    if changed {
        Manifest::build(&target_path)?.write(&target_path)?;
    }
    Ok(())
}

/// The on-disk path of a referenced blob, for handing to the OS default app.
/// Errors if the reference is malformed or the blob is not present locally (it
/// may live only in a backup/remote) — the caller surfaces that as a warning.
pub fn blob_path(vault: &Path, reference: &str) -> Result<PathBuf, StoreError> {
    let hash = parse_ref(reference)?;
    let store = BlobStore::new(vault);
    if !store.exists(&hash) {
        return Err(StoreError::Io(format!("blob not present locally: {hash}")));
    }
    Ok(store.path_for(&hash))
}

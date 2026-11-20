//! **Media from any site, kept as one final file** (`decisions.md` 2026-11-17; plan:
//! `web-media-privacy-plan-2026-11-17.md`, parts D and E).
//!
//! The pure half, shared by every device: what to ask yt-dlp for a choice (video, the audio as the
//! site serves it, the audio as MP3), how to read what it says while it works and what it says about
//! the media, and the notes the kept file becomes: **a song** in Music (`log: music`, as *Add a song*
//! makes, `ui/src/lib/music/songDraft.ts`) or **a video** (`log: video`), and a playlist's next entry
//! (`ui/src/lib/music/playlist.ts`'s list). No process, no network, no files: the computer runs the
//! tool from `fm-serve`, the phone from its shell.

use crate::dto::ObjectMeta;
use crate::log::NoteDraft;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The `log` kind of a video.
pub const VIDEO: &str = "video";

/// What the person chose to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum What {
    /// The video, at most `quality` lines high, video and sound joined without re-encoding.
    Video,
    /// The sound, as the site serves it (AAC or Opus): no conversion, the best quality.
    Audio,
    /// The sound as MP3 (an encode).
    Mp3,
}

/// Where the kept file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Into {
    /// A song in Music (a playlist chosen, or none).
    Music,
    /// A video in Videos (a collection chosen, or none).
    Videos,
    /// Attached to a note (the note looked at last), as a picture is.
    Note,
}

/// The MP3 quality setting: `"v2"` (variable, about 190 kbps, the default) or a constant rate.
pub fn mp3_quality(setting: &str) -> &'static str {
    match setting {
        "192" => "192K",
        "256" => "256K",
        "320" => "320K",
        _ => "2",
    }
}

/// yt-dlp's arguments for a choice, writing into `dir` (one file, `media.<ext>`). `ffmpeg` is the
/// folder holding ffmpeg and ffprobe, when there is one. The two `FM…` lines it prints are read by
/// [`read_line`].
pub fn ytdlp_args(
    what: What,
    quality: u32,
    mp3: &str,
    dir: &str,
    ffmpeg: Option<&str>,
    url: &str,
    meta: &Meta,
) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |s: &str| a.push(s.to_string());
    match what {
        What::Video => {
            let h = if quality == 0 { 1080 } else { quality };
            push("-f");
            push(&format!("bv*[height<={h}]+ba/b[height<={h}]/bv*+ba/b"));
            push("--merge-output-format");
            push("mp4/mkv");
        }
        // AAC first: it plays everywhere; Opus when it is all there is.
        What::Audio => {
            push("-f");
            push("ba[ext=m4a]/ba[acodec^=mp4a]/ba/b");
        }
        What::Mp3 => {
            push("-f");
            push("ba/b");
            push("-x");
            push("--audio-format");
            push("mp3");
            push("--audio-quality");
            push(mp3_quality(mp3));
        }
    }
    for s in [
        "--no-playlist",
        "--no-part",
        "--no-mtime",
        "--no-cache-dir",
        "--newline",
        "--no-colors",
        // `--print` silences the rest; the progress lines are still wanted.
        "--progress",
        // The details and the cover go inside the file itself: any player shows them.
        "--embed-metadata",
        "--embed-thumbnail",
        "--convert-thumbnails",
        "jpg",
        "--progress-template",
        "download:FMPROG %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s",
        "--print",
        "after_move:FMFILE %(filepath)s",
        "--print",
        "after_move:FMINFO %(.{title,uploader,channel,artist,creator,track,album,release_year,upload_date,duration,webpage_url,extractor_key,id})j",
    ] {
        push(s);
    }
    if let Some(f) = ffmpeg {
        push("--ffmpeg-location");
        push(f);
    }
    for s in meta_args(meta) {
        push(&s);
    }
    push("-o");
    push(&format!("{dir}/media.%(ext)s"));
    push("--");
    push(url);
    a
}

/// **The phone's runs, with no ffmpeg in the app** (`decisions.md` 2026-11-17, *structure, a size
/// budget*): the sound as AAC (kept as it is, or made MP3 afterwards by `fm-audio`), and a video as
/// its picture (H.264 in MP4) and its sound in two runs, which Android's `MediaMuxer` joins. Answers
/// the runs and whether they are to be joined. Each run writes `<dir>/<name>.<ext>` and prints the
/// same `FM…` lines as [`ytdlp_args`].
pub fn phone_runs(what: What, quality: u32, dir: &str, url: &str, meta: &Meta) -> Value {
    let h = if quality == 0 { 1080 } else { quality };
    let run = |format: &str, name: &str| -> Vec<String> {
        let mut a: Vec<String> = vec!["-f".into(), format.into()];
        for s in [
            "--no-playlist",
            // A page with several recordings (a Wikipedia article): the first, not all of them.
            "--playlist-items",
            "1",
            "--no-part",
            "--no-mtime",
            "--no-cache-dir",
            "--newline",
            "--no-colors",
            "--progress",
            // Nothing that needs ffmpeg: no fix-ups, no embedding (the tags are written afterwards).
            "--fixup",
            "never",
            "--progress-template",
            "download:FMPROG %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s",
            "--print",
            "after_move:FMFILE %(filepath)s",
            "--print",
            "after_move:FMINFO %(.{title,uploader,channel,artist,creator,track,album,release_year,upload_date,duration,webpage_url,extractor_key,id,thumbnail})j",
        ] {
            a.push(s.into());
        }
        a.extend(meta_args(meta));
        a.push("-o".into());
        a.push(format!("{dir}/{name}.%(ext)s"));
        a.push("--".into());
        a.push(url.into());
        a
    };
    match what {
        // As the site serves it, whatever kind: kept as it is. AAC first, which plays everywhere.
        What::Audio => {
            json!({ "runs": [run("ba[ext=m4a]/ba[acodec^=mp4a]/ba/b", "media")], "join": false })
        }
        // What `fm-audio` reads, an MP3 first (kept as it is); Opus last, said plainly if it fails.
        What::Mp3 => json!({
            // `/b` last: a site that names no codec (Wikimedia) has no "best audio" by yt-dlp's
            // reckoning, and without it nothing at all is chosen (found on the phone, 2026-11-17).
            "runs": [run("ba[ext=mp3]/ba[acodec=mp3]/ba[acodec^=mp4a]/ba[acodec=vorbis]/ba[acodec=flac]/ba/b", "media")],
            "join": false
        }),
        // The picture (MP4 first, else WebM, else a file with both); its sound only if it has none,
        // in the same kind of container (Android joins MP4 with AAC, WebM with Opus or Vorbis).
        What::Video => json!({
            "runs": [run(&format!("bv*[ext=mp4][vcodec^=avc1][height<={h}]/bv*[ext=webm][height<={h}]/b[height<={h}]/bv*/b"), "video")],
            "join": true,
            "audio": {
                "mp4": run("ba[ext=m4a]/ba[acodec^=mp4a]", "audio"),
                "webm": run("ba[ext=webm]/ba[acodec=opus]/ba[acodec=vorbis]", "audio"),
            }
        }),
    }
}

/// What a line of yt-dlp's output says.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// How far the download is: a fraction, when the size is known.
    Progress(Option<f64>),
    /// It is converting or joining (ffmpeg at work).
    Converting,
    /// The finished file.
    File(String),
    /// The media's details.
    Info(Value),
    Other,
}

pub fn read_line(line: &str) -> Line {
    let l = line.trim();
    if let Some(rest) = l.strip_prefix("FMPROG ") {
        let n: Vec<Option<f64>> = rest.split_whitespace().map(|x| x.parse::<f64>().ok()).collect();
        let done = n.first().copied().flatten();
        let total = n.get(1).copied().flatten().or_else(|| n.get(2).copied().flatten());
        return Line::Progress(match (done, total) {
            (Some(d), Some(t)) if t > 0.0 => Some((d / t).clamp(0.0, 1.0)),
            _ => None,
        });
    }
    if let Some(p) = l.strip_prefix("FMFILE ") {
        return Line::File(p.to_string());
    }
    if let Some(j) = l.strip_prefix("FMINFO ") {
        return serde_json::from_str(j).map(Line::Info).unwrap_or(Line::Other);
    }
    if l.starts_with("[ExtractAudio]")
        || l.starts_with("[Merger]")
        || l.starts_with("[EmbedThumbnail]")
    {
        return Line::Converting;
    }
    Line::Other
}

/// yt-dlp's last word on a failure, as a person reads it.
pub fn explain(stderr: &str) -> String {
    let last = stderr.lines().rev().find(|l| l.starts_with("ERROR:")).unwrap_or("").trim();
    let low = last.to_lowercase();
    if low.contains("drm") {
        return "This media is protected (DRM): it cannot be kept.".into();
    }
    if low.contains("unsupported url") {
        return "No media was found at this address.".into();
    }
    if low.contains("sign in") || low.contains("login") || low.contains("private") {
        return "The site asks to sign in for this media.".into();
    }
    if low.contains("unable to extract") || low.contains("http error 403") {
        return "This site changed how it serves media. Try again after the media tools update \
                (they do so by themselves)."
            .into();
    }
    if last.is_empty() {
        // No `ERROR:` line (the phone's runner failing around yt-dlp, say): its own last words,
        // rather than a sentence that hides them (the owner on 0.51's phone: only *"The media could
        // not be fetched"*).
        // The first thing said that is not a warning (on the phone: the failure's own name, then
        // yt-dlp's last lines).
        let said = stderr
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("WARNING"))
            .unwrap_or("");
        if said.is_empty() {
            return "The media could not be fetched.".into();
        }
        let said: String = said.chars().take(240).collect();
        return format!("The media could not be fetched: {said}");
    }
    last.trim_start_matches("ERROR:").trim().to_string()
}

/// The text of a detail, if it has one.
fn text(info: &Value, k: &str) -> Option<String> {
    info.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// A file name for the kept file: `Artist - Title.ext`, without characters a file system refuses.
pub fn file_name(info: &Value, ext: &str) -> String {
    let title =
        text(info, "track").or_else(|| text(info, "title")).unwrap_or_else(|| "media".into());
    let who =
        text(info, "artist").or_else(|| text(info, "uploader")).or_else(|| text(info, "channel"));
    let base = match who {
        Some(w) if !title.contains(&w) => format!("{w} - {title}"),
        _ => title,
    };
    let clean: String = base
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean: String = clean.chars().take(120).collect();
    format!("{}.{}", clean.trim_matches('.'), ext.trim_start_matches('.'))
}

fn label(t: &str) -> String {
    t.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

fn year(info: &Value) -> Option<u64> {
    info.get("release_year")
        .and_then(Value::as_u64)
        .or_else(|| text(info, "upload_date").and_then(|d| d.get(..4).and_then(|y| y.parse().ok())))
}

/// The details every kept file's note carries: where it came from.
fn source(props: &mut BTreeMap<String, Value>, info: &Value, media: &str, page: &str) {
    let from = text(info, "webpage_url").unwrap_or_else(|| media.to_string());
    props.insert("source".into(), json!(from));
    if !page.is_empty() && page != from {
        props.insert("page".into(), json!(page));
    }
}

/// **A song note** for a kept audio file, as *Add a song* makes one (`songDraft.ts`): the asset in
/// `audio`, the details from the file's own tags first (read at ingest) and yt-dlp's after.
pub fn song_draft(asset: &ObjectMeta, info: &Value, media: &str, page: &str) -> NoteDraft {
    let audio = asset.assets.first().cloned().unwrap_or_default();
    let tag = |k: &str| {
        asset.props.get(k).and_then(Value::as_str).map(str::to_string).filter(|s| !s.is_empty())
    };
    let title = tag("tag_title")
        .or_else(|| text(info, "track"))
        .or_else(|| text(info, "title"))
        .unwrap_or_else(|| "Untitled".into());
    let mut props: BTreeMap<String, Value> = BTreeMap::new();
    props.insert("log".into(), json!(crate::music::MUSIC));
    props.insert("audio".into(), json!(audio));
    let artist = tag("artist")
        .or_else(|| text(info, "artist"))
        .or_else(|| text(info, "creator"))
        .or_else(|| text(info, "uploader"))
        .or_else(|| text(info, "channel"));
    if let Some(a) = artist {
        props.insert("artist".into(), json!(a));
    }
    if let Some(a) = tag("album").or_else(|| text(info, "album")) {
        props.insert("album".into(), json!(a));
    }
    if let Some(y) = year(info) {
        props.insert("year".into(), json!(y));
    }
    if let Some(d) = asset.props.get("duration_s").filter(|v| v.is_number()) {
        props.insert("duration_s".into(), d.clone());
    } else if let Some(d) = info.get("duration").and_then(Value::as_f64) {
        props.insert("duration_s".into(), json!(d.round() as u64));
    }
    if let Some(c) = asset.props.get("cover").filter(|v| v.is_string()) {
        props.insert("cover".into(), c.clone());
    }
    source(&mut props, info, media, page);
    NoteDraft {
        body: format!("![{}](asset:{})\n", label(&title), audio.replacen("sha256:", "sha256-", 1)),
        title,
        props,
        ..Default::default()
    }
}

/// **A video note**: `log: video`, the asset in `video`, as a song keeps its audio.
pub fn video_draft(asset: &ObjectMeta, info: &Value, media: &str, page: &str) -> NoteDraft {
    let video = asset.assets.first().cloned().unwrap_or_default();
    let title = text(info, "title").unwrap_or_else(|| "Untitled video".into());
    let mut props: BTreeMap<String, Value> = BTreeMap::new();
    props.insert("log".into(), json!(VIDEO));
    props.insert("video".into(), json!(video));
    if let Some(a) = text(info, "uploader").or_else(|| text(info, "channel")) {
        props.insert("channel".into(), json!(a));
    }
    if let Some(y) = year(info) {
        props.insert("year".into(), json!(y));
    }
    if let Some(d) = info.get("duration").and_then(Value::as_f64) {
        props.insert("duration_s".into(), json!(d.round() as u64));
    }
    source(&mut props, info, media, page);
    NoteDraft {
        body: format!("![{}](asset:{})\n", label(&title), video.replacen("sha256:", "sha256-", 1)),
        title,
        props,
        ..Default::default()
    }
}

/// A playlist's (or collection's) body with one more entry at the end of its list, numbered after
/// the last (`playlist.ts`: `N. [Title](note:<id>)`; the other lines kept where they are).
pub fn playlist_with(body: &str, id: &str, title: &str) -> String {
    let is_entry = |l: &str| {
        let t = l.trim_start();
        let digits = t.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && t[digits..].starts_with(['.', ')']) && t.contains("](note:")
    };
    let lines: Vec<&str> = body.lines().collect();
    let entry = |n: usize| format!("{n}. [{}](note:{id})", label(title));
    match lines.iter().rposition(|l| is_entry(l)) {
        Some(last) => {
            let n = lines.iter().filter(|l| is_entry(l)).count() + 1;
            let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            out.insert(last + 1, entry(n));
            out.join("\n") + "\n"
        }
        None => {
            let kept = body.trim_end();
            let lead = if kept.is_empty() { String::new() } else { format!("{kept}\n\n") };
            format!("{lead}{}\n", entry(1))
        }
    }
}

/// What the sheet shows of an address before anything is fetched: yt-dlp's `-J` answer, summed up.
/// File kinds that hold a picture, for a format whose codec is not named.
const VIDEO_EXTS: &[&str] =
    &["webm", "mp4", "m4v", "mkv", "mov", "ogv", "avi", "mpg", "mpeg", "3gp"];
/// File kinds that are sound alone.
const AUDIO_EXTS: &[&str] =
    &["mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav", "mid", "midi"];

pub fn probe_summary(info: &Value) -> Value {
    // **No list of formats: the item is its one format** (an older yt-dlp gives Commons' file so: its
    // address and its kind, nothing else; found on a fresh phone, 2026-11-17).
    let formats = match info["formats"].as_array() {
        Some(f) if !f.is_empty() => f.clone(),
        _ if info["url"].is_string() || info["ext"].is_string() => vec![info.clone()],
        _ => vec![],
    };
    // **A picture is video, named or not**: yt-dlp says `none` for sound alone, and leaves the codec
    // unsaid where a site does not tell it (Commons' original file, for an older yt-dlp, as the phone's
    // own copy is until it updates: "The video" was refused there, 2026-11-17). A height says picture.
    let video = |f: &Value| match f["vcodec"].as_str() {
        Some("none") => false,
        Some(_) => true,
        None => {
            f["height"].as_u64().is_some_and(|h| h > 0)
                || f["ext"].as_str().is_some_and(|e| VIDEO_EXTS.contains(&e))
        }
    };
    let audio = |f: &Value| match f["acodec"].as_str() {
        Some("none") => false,
        Some(_) => true,
        // Unsaid: a sound file, or a picture file (which most often carries its sound).
        None => {
            f["ext"].as_str().is_some_and(|e| AUDIO_EXTS.contains(&e) || VIDEO_EXTS.contains(&e))
        }
    };
    let mut heights: Vec<u64> =
        formats.iter().filter(|f| video(f)).filter_map(|f| f["height"].as_u64()).collect();
    heights.sort_unstable();
    heights.dedup();
    json!({
        "title": info["title"], "uploader": info["uploader"].as_str().or(info["channel"].as_str()),
        "duration": info["duration"], "thumbnail": info["thumbnail"], "site": info["extractor_key"],
        "live": info["is_live"].as_bool().unwrap_or(false),
        "video": formats.iter().any(video) || info["vcodec"].as_str().is_some_and(|c| c != "none"),
        "audio": formats.iter().any(audio) || formats.is_empty(),
        "heights": heights,
    })
}

/// **What the page said the media is** (2026-11-20, Bandcamp): its title, artist and album, for a
/// file whose address says nothing — a Bandcamp track's stream, taken from the page because
/// Bandcamp's own pages answer the phone's yt-dlp with a challenge it cannot pass. Empty: yt-dlp's
/// own reading stands.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    /// The media's own page (the track's), kept as the note's `source` instead of the file's
    /// address, which expires.
    #[serde(default)]
    pub source: String,
}

/// [`Meta`] as yt-dlp's `--parse-metadata`, set before anything is written (the file's tags, the
/// `FMINFO` line the note is made from). Each value is literal text: given its moment
/// (`pre_process:`, so a title that begins `video:` is not taken for one), behind an empty template
/// (so a one-word title is not taken for a field's name), with `%` and `:` escaped and a trailing
/// backslash dropped (it would escape the separator).
pub fn meta_args(meta: &Meta) -> Vec<String> {
    let mut a = Vec::new();
    for (field, value) in [
        ("title", &meta.title),
        ("artist", &meta.artist),
        ("album", &meta.album),
        ("webpage_url", &meta.source),
    ] {
        let v =
            value.trim().trim_end_matches('\\').trim_end().replace('%', "%%").replace(':', "\\:");
        if v.is_empty() {
            continue;
        }
        a.push("--parse-metadata".into());
        a.push(format!("pre_process:%(fm_none|)s{v}:%({field})s"));
    }
    a
}

/// **What the person asks for** (the Save media sheet's request, on every device).
#[derive(Debug, Clone, Deserialize)]
pub struct Ask {
    /// The media's address (an embedded player's, a file's, the page's).
    pub url: String,
    /// The page it was found on.
    #[serde(default)]
    pub page: String,
    pub what: What,
    /// The video's height at most (0: 1080).
    #[serde(default)]
    pub quality: u32,
    /// The MP3 setting (`v2`, `192`, `256`, `320`).
    #[serde(default)]
    pub mp3: String,
    pub into: Into,
    /// A playlist (Music) or collection (Videos) note to add it to; or `new:<name>` for a new one.
    #[serde(default)]
    pub list: String,
    /// The note to attach it to (`into: note`).
    #[serde(default)]
    pub note: String,
    /// The vault it goes in (empty: the first).
    #[serde(default)]
    pub vault: String,
    /// What the page said the media is, when its address says nothing ([`Meta`]).
    #[serde(default)]
    pub meta: Meta,
}

/// A dispatch on this device's own behalf, its JSON answer.
fn call(app: &crate::App, host: &dyn crate::Host, cmd: &str, args: Value) -> Result<Value, String> {
    let bytes = crate::dispatch::dispatch(cmd, &args, &[], app, host)?.into_bytes();
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// The vault's name and folder (the first, when none is named).
pub fn vault_of(
    app: &crate::App,
    host: &dyn crate::Host,
    name: &str,
) -> Result<(String, std::path::PathBuf), String> {
    let list = call(app, host, "list_vaults", json!({}))?;
    let all = list.as_array().cloned().unwrap_or_default();
    let v = all
        .iter()
        .find(|v| name.is_empty() || v["name"] == name)
        .ok_or_else(|| format!("no vault called {name}"))?;
    Ok((
        v["name"].as_str().unwrap_or("").to_string(),
        std::path::PathBuf::from(v["path"].as_str().unwrap_or("")),
    ))
}

/// **The finished file into the vault, and the note it becomes** (every device). `file` is moved
/// into the vault's staging place for `session` (`.fm-ingest/<session>/part`; copied when it is on
/// another disk), stored content-addressed (`ingest_finish`: a file already there is not stored
/// twice), and becomes a song, a video, or an attachment of a note; then joins its list. Answers
/// the note's id. The staging place is gone afterwards, whatever happened.
pub fn keep(
    app: &crate::App,
    host: &dyn crate::Host,
    vault: &str,
    file: &std::path::Path,
    session: &str,
    ask: &Ask,
    info: &Value,
) -> Result<String, String> {
    let (vault, vpath) = vault_of(app, host, vault)?;
    let stage = vpath.join(".fm-ingest").join(session);
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let part = stage.join("part");
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("bin").to_string();
    let moved = std::fs::rename(file, &part).or_else(|_| {
        std::fs::copy(file, &part).map(|_| ()).and_then(|_| std::fs::remove_file(file))
    });
    let result = moved.map_err(|e| e.to_string()).and_then(|_| {
        for e in std::fs::read_dir(&stage).into_iter().flatten().flatten() {
            if e.path() != part {
                let _ =
                    std::fs::remove_dir_all(e.path()).or_else(|_| std::fs::remove_file(e.path()));
            }
        }
        let asset: ObjectMeta = serde_json::from_value(call(
            app,
            host,
            "ingest_finish",
            json!({ "vault": vault, "session": session, "name": file_name(info, &ext) }),
        )?)
        .map_err(|e| e.to_string())?;
        file_it(app, host, &vault, ask, &asset, info)
    });
    let _ = std::fs::remove_dir_all(&stage);
    result
}

/// The note the kept file becomes; answers its id.
fn file_it(
    app: &crate::App,
    host: &dyn crate::Host,
    vault: &str,
    ask: &Ask,
    asset: &ObjectMeta,
    info: &Value,
) -> Result<String, String> {
    let made = |draft: NoteDraft| -> Result<(String, String), String> {
        let title = draft.title.clone();
        let out = call(app, host, "create_notes", json!({ "vault": vault, "notes": [draft] }))?;
        let id = out[0]["id"].as_str().ok_or("the note was not made")?.to_string();
        Ok((id, title))
    };
    let (id, title) = match ask.into {
        Into::Note => {
            let r =
                asset.assets.first().cloned().unwrap_or_default().replacen("sha256:", "sha256-", 1);
            let name = info["title"].as_str().unwrap_or("media");
            call(
                app,
                host,
                "put",
                json!({ "to": { "note": ask.note }, "md": format!("![{}](asset:{r})\n", label(name)) }),
            )?;
            return Ok(ask.note.clone());
        }
        Into::Music => made(song_draft(asset, info, &ask.url, &ask.page))?,
        Into::Videos => made(video_draft(asset, info, &ask.url, &ask.page))?,
    };
    if !ask.list.is_empty() {
        let tag =
            if ask.into == Into::Music { crate::music::PLAYLIST } else { crate::video::COLLECTION };
        if let Some(name) = ask.list.strip_prefix("new:") {
            let draft = NoteDraft {
                title: name.trim().to_string(),
                body: playlist_with("", &id, &title),
                tags: vec![tag.to_string()],
                ..Default::default()
            };
            call(app, host, "create_notes", json!({ "vault": vault, "notes": [draft] }))?;
        } else {
            let list = call(app, host, "get", json!({ "id": ask.list }))?;
            let body = list["body"].as_str().unwrap_or("");
            let base = list["version"].as_str().unwrap_or("");
            call(
                app,
                host,
                "update_body",
                json!({ "id": ask.list, "body": playlist_with(body, &id, &title), "base": base }),
            )?;
        }
    }
    Ok(id)
}

/// One entry of a playlist file: what a player shows, its length, and where the file is from the
/// `playlists/` folder.
pub struct M3uEntry {
    pub title: String,
    pub artist: String,
    pub seconds: Option<u64>,
    pub path: String,
}

/// **A playlist as a file other players read** (extended M3U, UTF-8: `music-plan`'s one format that
/// travels), its paths relative to the file, so the vault's folder can move.
pub fn m3u8(name: &str, entries: &[M3uEntry]) -> String {
    let mut out = format!("#EXTM3U\n#PLAYLIST:{name}\n# Written by formicaria from the note \"{name}\"; edit the note, not this file.\n");
    for e in entries {
        let shown = if e.artist.is_empty() {
            e.title.clone()
        } else {
            format!("{} - {}", e.artist, e.title)
        };
        let secs = e.seconds.map_or(-1, |s| s as i64);
        out.push_str(&format!("#EXTINF:{secs},{}\n{}\n", shown.replace('\n', " "), e.path));
    }
    out
}

/// A playlist's file name: its title, without characters a file system refuses.
pub fn m3u8_name(title: &str) -> String {
    let t: String = title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{}.m3u8", if t.is_empty() { "Playlist".into() } else { t })
}

/// **Every playlist and collection of a vault, written as `.m3u8` beside its media**
/// (`<vault>/playlists/`), so any player pointed at the vault shows them; one copy of each file,
/// the vault's own. Files formicaria wrote and whose note is gone are removed; others are left.
/// Answers how many were written.
pub fn export_lists(
    store: &dyn fm_core::Store,
    vault: &std::path::Path,
    name: &str,
) -> Result<usize, String> {
    // Several vaults answer through one store: only this vault's lists go in its folder.
    let here = |id: &str| {
        id.parse::<fm_model::Id>().ok().and_then(|i| store.get(i).ok().flatten()).is_some_and(|o| {
            let v = ObjectMeta::from(&o).vault;
            name.is_empty() || v.is_empty() || v == name
        })
    };
    let lists: Vec<(crate::music::Playlist, &str)> = crate::music::playlists(store)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| (p, "audio"))
        .chain(
            crate::video::collections(store)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|p| (p, "video")),
        )
        .filter(|(p, _)| here(&p.id))
        .collect();
    let dir = vault.join("playlists");
    let blobs = fm_core::blob::BlobStore::new(vault);
    let mut wanted = std::collections::BTreeSet::new();
    let mut written = 0;
    for (list, prop) in &lists {
        let mut entries = Vec::new();
        for id in &list.songs {
            let Ok(id) = id.parse::<fm_model::Id>() else { continue };
            let Some(o) = store.get(id).map_err(|e| e.to_string())? else { continue };
            let meta = ObjectMeta::from(&o);
            let file = meta.props.get(*prop).and_then(Value::as_str).unwrap_or("");
            let Some(hash) = file.strip_prefix("sha256:") else { continue };
            if hash.len() < 4 || !blobs.exists(hash) {
                continue;
            }
            let full = blobs.path_for(hash);
            let rel = full
                .strip_prefix(vault)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let text =
                |k: &str| meta.props.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            entries.push(M3uEntry {
                title: meta.title.clone().unwrap_or_default(),
                artist: {
                    let a = text("artist");
                    if a.is_empty() {
                        text("channel")
                    } else {
                        a
                    }
                },
                seconds: meta.props.get("duration_s").and_then(Value::as_u64),
                path: format!("../{rel}"),
            });
        }
        let name = m3u8_name(&list.title);
        let text = m3u8(&list.title, &entries);
        let path = dir.join(&name);
        wanted.insert(name);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(&path, text).map_err(|e| e.to_string())?;
            written += 1;
        }
    }
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let ours =
            std::fs::read_to_string(e.path()).is_ok_and(|t| t.contains("# Written by formicaria"));
        if n.ends_with(".m3u8") && ours && !wanted.contains(&n) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(props: Value) -> ObjectMeta {
        let mut m: ObjectMeta = serde_json::from_value(json!({
            "id": "01J0000000000000000000000A", "type": "asset", "title": null, "preview": "",
            "status": null, "due": null, "start": null, "hard": false, "created": "", "updated": "",
            "tags": [], "assets": ["sha256:abc"], "props": {}, "vault": ""
        }))
        .unwrap();
        m.props = serde_json::from_value(props).unwrap();
        m
    }

    #[test]
    fn each_choice_asks_for_only_what_it_keeps() {
        let a = ytdlp_args(
            What::Audio,
            0,
            "v2",
            "/tmp/j",
            None,
            "https://v.example/w?v=1",
            &Meta::default(),
        );
        let f = a.iter().position(|x| x == "-f").unwrap();
        assert!(a[f + 1].starts_with("ba"), "the sound alone, never the video");
        assert!(!a.contains(&"-x".to_string()), "as served: no conversion");
        let m = ytdlp_args(
            What::Mp3,
            0,
            "320",
            "/tmp/j",
            Some("/t/ff"),
            "https://v.example/w",
            &Meta::default(),
        );
        assert!(m.windows(2).any(|w| w == ["--audio-quality", "320K"]));
        assert!(m.windows(2).any(|w| w == ["--ffmpeg-location", "/t/ff"]));
        let v = ytdlp_args(
            What::Video,
            720,
            "v2",
            "/tmp/j",
            None,
            "https://v.example/w",
            &Meta::default(),
        );
        assert!(v.iter().any(|x| x.contains("height<=720")));
        // The address last, after `--`, so an address can never be read as an option.
        assert_eq!(&v[v.len() - 2..], ["--", "https://v.example/w"]);
        assert!(v.contains(&"/tmp/j/media.%(ext)s".to_string()));
    }

    #[test]
    fn an_item_with_no_formats_is_read_by_its_own_kind() {
        // Commons' file as yt-dlp 2025.11.12 (the phone's own, before it updates) gives it.
        let commons = json!({ "title": "Ant Bed With Ants.webm - Wikimedia Commons",
            "url": "https://upload.wikimedia.org/wikipedia/commons/a/ab/Ant_Bed_With_Ants.webm",
            "ext": "webm", "extractor_key": "Wikimedia" });
        let s = probe_summary(&commons);
        assert_eq!(s["video"], true);
        assert_eq!(s["audio"], true);
        let song = json!({ "title": "A song", "url": "https://x.example/a.ogg", "ext": "ogg" });
        assert_eq!(probe_summary(&song)["video"], false);
        assert_eq!(probe_summary(&song)["audio"], true);
    }

    #[test]
    fn a_picture_with_no_codec_named_is_video_and_sound_alone_is_not() {
        // Commons' original file as an older yt-dlp lists it: a height, no codec named.
        let commons = json!({ "title": "Ant Bed", "formats": [
            { "format_id": "source", "ext": "webm", "vcodec": null, "acodec": null, "height": 1080 }
        ]});
        let s = probe_summary(&commons);
        assert_eq!(s["video"], true);
        assert_eq!(s["heights"], json!([1080]));
        let song = json!({ "title": "A song", "formats": [
            { "format_id": "a", "ext": "mp3", "vcodec": "none", "acodec": "mp3" },
            { "format_id": "b", "ext": "ogg", "vcodec": null, "acodec": "vorbis" }
        ]});
        assert_eq!(probe_summary(&song)["video"], false);
    }

    #[test]
    fn the_phone_asks_for_nothing_ffmpeg_would_do() {
        let p = phone_runs(What::Mp3, 0, "/c/1", "https://v.example/w", &Meta::default());
        let run: Vec<String> = serde_json::from_value(p["runs"][0].clone()).unwrap();
        assert!(
            run.windows(2).any(|w| w[0] == "-f" && w[1].starts_with("ba[ext=mp3]")),
            "an MP3 first"
        );
        assert!(run.windows(2).any(|w| w == ["--playlist-items", "1"]), "one item of a page");
        let all = serde_json::to_string(&p).unwrap();
        for ffmpeg in [
            "\"-x\"",
            "--embed-metadata",
            "--embed-thumbnail",
            "--merge-output-format",
            "--convert-thumbnails",
        ] {
            assert!(!all.contains(ffmpeg), "{ffmpeg}");
        }
        // **Every choice ends in "whatever there is"**: a site that labels nothing still gives a file.
        for w in [What::Mp3, What::Audio, What::Video] {
            let f = phone_runs(w, 0, "/c/1", "https://v.example/w", &Meta::default())["runs"][0][1]
                .as_str()
                .unwrap()
                .to_string();
            assert!(f.ends_with("/b"), "{w:?}: {f}");
        }
        let a = phone_runs(What::Audio, 0, "/c/1", "https://v.example/w", &Meta::default());
        assert!(a["runs"][0][1].as_str().unwrap().ends_with("/ba/b"), "as served: any kind");
        let v = phone_runs(What::Video, 720, "/c/1", "https://v.example/w", &Meta::default());
        assert_eq!(v["join"], true);
        assert!(
            v["runs"][0][1].as_str().unwrap().contains("bv*[ext=webm][height<=720]"),
            "WebM too"
        );
        assert!(v["audio"]["mp4"][1].as_str().unwrap().starts_with("ba[ext=m4a]"));
        assert!(v["audio"]["webm"][1].as_str().unwrap().starts_with("ba[ext=webm]"));
        assert!(v["audio"]["webm"].as_array().unwrap().iter().any(|x| x == "/c/1/audio.%(ext)s"));
    }

    #[test]
    fn its_lines_are_read() {
        assert_eq!(read_line("FMPROG 50 200 NA"), Line::Progress(Some(0.25)));
        assert_eq!(read_line("FMPROG 50 NA 100"), Line::Progress(Some(0.5)));
        assert_eq!(read_line("FMPROG 50 NA NA"), Line::Progress(None));
        assert_eq!(read_line("FMFILE /tmp/j/media.m4a"), Line::File("/tmp/j/media.m4a".into()));
        assert!(matches!(read_line(r#"FMINFO {"title":"T"}"#), Line::Info(_)));
        assert_eq!(read_line("[ExtractAudio] Destination: x.mp3"), Line::Converting);
        assert_eq!(read_line("[youtube] abc: Downloading"), Line::Other);
    }

    #[test]
    fn a_failure_is_said_plainly() {
        assert!(explain("x\nERROR: [x] This video is DRM protected").contains("DRM"));
        assert!(explain("ERROR: Unsupported URL: https://a.example").contains("No media"));
        assert!(explain("").contains("could not be fetched"));
        // Whatever was said, when it is not yt-dlp's ERROR line.
        assert_eq!(
            explain("IllegalStateException: yt-dlp finished without a file\n[youtube] abc: Downloading\n"),
            "The media could not be fetched: IllegalStateException: yt-dlp finished without a file"
        );
        assert!(explain("WARNING: no JS runtime\nYoutubeDLException: python could not start")
            .ends_with("python could not start"));
    }

    #[test]
    fn a_kept_file_is_named_for_people() {
        let i = json!({"title": "Leafcutters: a year / at night", "uploader": "Ant Channel"});
        assert_eq!(file_name(&i, "mp3"), "Ant Channel - Leafcutters a year at night.mp3");
        assert_eq!(file_name(&json!({}), "m4a"), "media.m4a");
    }

    #[test]
    fn a_song_is_the_note_add_a_song_makes() {
        let a = asset(json!({"artist": "Tagged Artist", "duration_s": 201, "cover": "sha256:c"}));
        let info = json!({"title": "Leafcutters (Live)", "uploader": "Ant Channel",
                          "upload_date": "20240105", "webpage_url": "https://v.example/w?v=1"});
        let d = song_draft(&a, &info, "https://v.example/embed/1", "https://news.example/a");
        assert_eq!(d.title, "Leafcutters (Live)");
        assert_eq!(d.props["log"], "music");
        assert_eq!(d.props["audio"], "sha256:abc");
        // The file's own tags first.
        assert_eq!(d.props["artist"], "Tagged Artist");
        assert_eq!(d.props["year"], 2024);
        assert_eq!(d.props["duration_s"], 201);
        assert_eq!(d.props["source"], "https://v.example/w?v=1");
        assert_eq!(d.props["page"], "https://news.example/a");
        assert_eq!(d.body, "![Leafcutters (Live)](asset:sha256-abc)\n");
    }

    #[test]
    fn a_video_is_a_logged_note_with_its_file() {
        let d = video_draft(
            &asset(json!({})),
            &json!({"title": "Ants", "duration": 61.6, "channel": "C"}),
            "https://v.example/x",
            "",
        );
        assert_eq!(d.props["log"], "video");
        assert_eq!(d.props["video"], "sha256:abc");
        assert_eq!(d.props["duration_s"], 62);
        assert_eq!(d.props["channel"], "C");
        assert!(!d.props.contains_key("page"));
    }

    #[test]
    fn a_playlist_file_is_extended_m3u_with_relative_paths() {
        let t = m3u8(
            "Road",
            &[M3uEntry {
                title: "One".into(),
                artist: "Band".into(),
                seconds: Some(201),
                path: "../blobs/sha256/ab/cd/abcd".into(),
            }],
        );
        assert!(t.starts_with("#EXTM3U\n#PLAYLIST:Road\n"));
        assert!(t.contains("#EXTINF:201,Band - One\n../blobs/sha256/ab/cd/abcd\n"));
        assert_eq!(m3u8_name("Road / trip: 2026"), "Road trip 2026.m3u8");
        assert_eq!(m3u8_name("  "), "Playlist.m3u8");
    }

    #[test]
    fn a_vaults_playlists_are_written_beside_its_media_and_a_gone_one_removed() {
        use crate::log::create_notes;
        let d = tempfile::tempdir().unwrap();
        let vault = d.path();
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let hash = fm_core::blob::BlobStore::new(vault).put_bytes(b"SONG").unwrap().hash;
        let mut store = fm_core::FileStore::open(vault).unwrap();
        let mut props = BTreeMap::new();
        props.insert("log".to_string(), json!("music"));
        props.insert("audio".to_string(), json!(format!("sha256:{hash}")));
        props.insert("artist".to_string(), json!("Band"));
        let song = create_notes(
            &mut store,
            "",
            &[NoteDraft { title: "One".into(), props, ..Default::default() }],
        )
        .unwrap();
        create_notes(
            &mut store,
            "",
            &[NoteDraft {
                title: "Road".into(),
                body: playlist_with("", &song[0].id, "One"),
                tags: vec!["playlist".into()],
                ..Default::default()
            }],
        )
        .unwrap();
        std::fs::create_dir_all(vault.join("playlists")).unwrap();
        std::fs::write(
            vault.join("playlists/Old.m3u8"),
            "#EXTM3U\n# Written by formicaria from the note \"Old\"\n",
        )
        .unwrap();
        std::fs::write(vault.join("playlists/Mine.m3u8"), "#EXTM3U\n").unwrap();
        assert_eq!(export_lists(&store, vault, "").unwrap(), 1);
        let t = std::fs::read_to_string(vault.join("playlists/Road.m3u8")).unwrap();
        assert!(t.contains("Band - One"));
        let path = t.lines().last().unwrap();
        assert_eq!(std::fs::read(vault.join("playlists").join(path)).unwrap(), b"SONG");
        assert!(!vault.join("playlists/Old.m3u8").exists(), "formicaria's, its note gone");
        assert!(vault.join("playlists/Mine.m3u8").exists(), "the person's own is left");
        assert_eq!(export_lists(&store, vault, "").unwrap(), 0, "unchanged: not written again");
    }

    #[test]
    fn a_playlist_gets_its_next_entry_at_the_end_of_its_list() {
        let body = "Songs for the road.\n\n1. [One](note:01J0000000000000000000000B)\n2. [Two](note:01J0000000000000000000000C)\n\nMore later.\n";
        let out = playlist_with(body, "01J0000000000000000000000D", "Three [live]");
        assert_eq!(
            out,
            "Songs for the road.\n\n1. [One](note:01J0000000000000000000000B)\n2. [Two](note:01J0000000000000000000000C)\n3. [Three \\[live\\]](note:01J0000000000000000000000D)\n\nMore later.\n"
        );
        assert_eq!(
            playlist_with("", "01J0000000000000000000000D", "X"),
            "1. [X](note:01J0000000000000000000000D)\n"
        );
    }

    /// What the page said the media is, handed to yt-dlp as literal text before the address
    /// (2026-11-20, Bandcamp's stream). Checked against yt-dlp 2026.10.26 by hand: a one-word title,
    /// a title beginning `video:` and one holding `%` and `:` all come out as given.
    #[test]
    fn what_the_page_said_goes_to_yt_dlp_as_literal_text() {
        let m = Meta { title: "Ants".into(), artist: "A: 100% \\".into(), ..Meta::default() };
        assert_eq!(
            meta_args(&m),
            vec![
                "--parse-metadata",
                "pre_process:%(fm_none|)sAnts:%(title)s",
                "--parse-metadata",
                "pre_process:%(fm_none|)sA\\: 100%%:%(artist)s",
            ]
        );
        assert!(meta_args(&Meta::default()).is_empty());
        let before_address = |a: &[String]| {
            a.iter().position(|s| s == "--parse-metadata").unwrap()
                < a.iter().position(|s| s == "--").unwrap()
        };
        assert!(before_address(&ytdlp_args(What::Mp3, 0, "v2", "/t", None, "https://x", &m)));
        let run: Vec<String> = serde_json::from_value(
            phone_runs(What::Mp3, 0, "/c", "https://x", &m)["runs"][0].clone(),
        )
        .unwrap();
        assert!(before_address(&run));
    }
}

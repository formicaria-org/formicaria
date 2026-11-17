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
    push("-o");
    push(&format!("{dir}/media.%(ext)s"));
    push("--");
    push(url);
    a
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
        return "The media could not be fetched.".into();
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
pub fn probe_summary(info: &Value) -> Value {
    let formats = info["formats"].as_array().cloned().unwrap_or_default();
    let video = |f: &Value| f["vcodec"].as_str().is_some_and(|c| c != "none");
    let audio = |f: &Value| f["acodec"].as_str().is_some_and(|c| c != "none");
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
        let a = ytdlp_args(What::Audio, 0, "v2", "/tmp/j", None, "https://v.example/w?v=1");
        let f = a.iter().position(|x| x == "-f").unwrap();
        assert!(a[f + 1].starts_with("ba"), "the sound alone, never the video");
        assert!(!a.contains(&"-x".to_string()), "as served: no conversion");
        let m = ytdlp_args(What::Mp3, 0, "320", "/tmp/j", Some("/t/ff"), "https://v.example/w");
        assert!(m.windows(2).any(|w| w == ["--audio-quality", "320K"]));
        assert!(m.windows(2).any(|w| w == ["--ffmpeg-location", "/t/ff"]));
        let v = ytdlp_args(What::Video, 720, "v2", "/tmp/j", None, "https://v.example/w");
        assert!(v.iter().any(|x| x.contains("height<=720")));
        // The address last, after `--`, so an address can never be read as an option.
        assert_eq!(&v[v.len() - 2..], ["--", "https://v.example/w"]);
        assert!(v.contains(&"/tmp/j/media.%(ext)s".to_string()));
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
}

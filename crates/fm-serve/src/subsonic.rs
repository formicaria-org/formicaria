//! **Your music in any Subsonic player** (`decisions.md` 2026-11-17, *media from any site*; plan
//! part E). This computer answers the part of the Subsonic API players use (Symfonium, DSub,
//! Tempo, Feishin…), so a phone or another computer streams the library **from the vault's own
//! files**: one copy, nothing synced or exported.
//!
//! Off until turned on (Settings → Music), and reachable from the network only while sharing is on
//! (the LAN listener). It has **its own password**, made here and shown in Settings; a Subsonic
//! player cannot use formicaria's pairing. It sees only Music (songs, albums, artists, playlists)
//! and streams only songs' files. Answers in JSON (`f=json`) or Subsonic's XML.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

const API: &str = "1.16.1";
pub const USER: &str = "formicaria";

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct Settings {
    pub on: bool,
    pub password: String,
}

fn path() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("subsonic.json"))
}

pub fn settings() -> Settings {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn new_password() -> String {
    let mut b = [0u8; 12];
    let _ = getrandom::fill(&mut b);
    const A: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    b.iter().map(|x| A[*x as usize % A.len()] as char).collect()
}

/// Turn it on or off; a password is made the first time, and again when asked.
pub fn set(on: Option<bool>, renew: bool) -> Result<Settings, String> {
    let mut s = settings();
    if let Some(on) = on {
        s.on = on;
    }
    if renew || s.password.is_empty() {
        s.password = new_password();
    }
    let p = path().ok_or("this machine has no settings folder")?;
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    }
    Ok(s)
}

// ── MD5, for Subsonic's token (`t = md5(password + salt)`): its one use here ───────────────────

pub fn md5_hex(input: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> =
        (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) =
        (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    let mut msg = input.to_vec();
    let bits = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> =
            chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|x| x.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

// ── the request ────────────────────────────────────────────────────────────────────────────────

fn query(target: &str, body: &[u8]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let q = target.split_once('?').map(|(_, q)| q.to_string()).unwrap_or_default();
    let form = String::from_utf8_lossy(body).to_string();
    for pair in q.split('&').chain(form.split('&')) {
        if let Some((k, v)) = pair.split_once('=') {
            out.insert(crate::percent_decode(k), crate::percent_decode(&v.replace('+', " ")));
        }
    }
    out
}

/// Is the player who it says? A token (`t`, `s`) or the password (`p`, plain or `enc:` hex).
pub fn authorized(q: &BTreeMap<String, String>, s: &Settings) -> bool {
    if !s.on || s.password.is_empty() || q.get("u").map(String::as_str) != Some(USER) {
        return false;
    }
    if let (Some(t), Some(salt)) = (q.get("t"), q.get("s")) {
        return t.eq_ignore_ascii_case(&md5_hex(format!("{}{salt}", s.password).as_bytes()));
    }
    match q.get("p") {
        Some(p) if p.starts_with("enc:") => {
            let hex = &p[4..];
            let bytes: Option<Vec<u8>> = (0..hex.len())
                .step_by(2)
                .map(|i| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok()))
                .collect();
            bytes.is_some_and(|b| b == s.password.as_bytes())
        }
        Some(p) => p == &s.password,
        None => false,
    }
}

// ── the answer ─────────────────────────────────────────────────────────────────────────────────

fn envelope(ok: bool, body: Value) -> Value {
    let mut r = Map::new();
    r.insert("status".into(), json!(if ok { "ok" } else { "failed" }));
    r.insert("version".into(), json!(API));
    r.insert("type".into(), json!("formicaria"));
    r.insert("openSubsonic".into(), json!(true));
    if let Value::Object(m) = body {
        r.extend(m);
    }
    json!({ "subsonic-response": Value::Object(r) })
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Subsonic's XML: an object's plain values are attributes, its objects and lists are elements.
pub fn to_xml(name: &str, v: &Value, out: &mut String) {
    match v {
        Value::Array(items) => {
            for i in items {
                to_xml(name, i, out);
            }
        }
        Value::Object(m) => {
            out.push('<');
            out.push_str(name);
            if name == "subsonic-response" {
                out.push_str(" xmlns=\"http://subsonic.org/restapi\"");
            }
            let mut kids = Vec::new();
            for (k, x) in m {
                match x {
                    Value::Object(_) | Value::Array(_) => kids.push((k, x)),
                    Value::Null => {}
                    Value::String(s) => out.push_str(&format!(" {k}=\"{}\"", esc(s))),
                    other => out.push_str(&format!(" {k}=\"{other}\"")),
                }
            }
            if kids.is_empty() {
                out.push_str("/>");
            } else {
                out.push('>');
                for (k, x) in kids {
                    to_xml(k, x, out);
                }
                out.push_str(&format!("</{name}>"));
            }
        }
        Value::String(s) => out.push_str(&format!("<{name}>{}</{name}>", esc(s))),
        other => out.push_str(&format!("<{name}>{other}</{name}>")),
    }
}

fn error(code: u32, message: &str) -> Value {
    envelope(false, json!({ "error": { "code": code, "message": message } }))
}

// ── the library, as Subsonic names things ───────────────────────────────────────────────────────

fn short(s: &str) -> String {
    fm_core::blob::sha256_hex(s.as_bytes())[..12].to_string()
}

struct Lib {
    songs: Vec<Value>,
}

fn text<'a>(o: &'a Value, k: &str) -> &'a str {
    o["props"][k].as_str().unwrap_or("")
}

fn song(o: &Value) -> Value {
    let artist = text(o, "artist");
    let album = text(o, "album");
    let by = if text(o, "album_artist").is_empty() { artist } else { text(o, "album_artist") };
    let cover = text(o, "cover");
    json!({
        "id": o["id"], "parent": format!("al-{}", short(&format!("{by}\u{1}{album}"))), "isDir": false,
        "title": o["title"], "album": album, "artist": artist,
        "albumId": format!("al-{}", short(&format!("{by}\u{1}{album}"))),
        "artistId": format!("ar-{}", short(by)),
        "track": o["props"]["track"], "year": o["props"]["year"], "genre": o["props"]["genre"],
        "duration": o["props"]["duration_s"],
        "coverArt": if cover.is_empty() { o["id"].clone() } else { json!(cover) },
        "type": "music", "isVideo": false,
    })
}

impl Lib {
    fn read(state: &crate::AppState) -> Lib {
        let out = fm_app::dispatch("music_library", &json!({}), &[], &state.app, &crate::Desktop)
            .map(|o| o.into_bytes())
            .unwrap_or_default();
        let notes: Vec<Value> = serde_json::from_slice(&out).unwrap_or_default();
        Lib { songs: notes.iter().map(song).collect() }
    }
    fn albums(&self) -> Vec<Value> {
        let mut by: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        for s in &self.songs {
            by.entry(s["albumId"].as_str().unwrap_or("").to_string()).or_default().push(s);
        }
        by.into_iter()
            .map(|(id, songs)| {
                let f = songs[0];
                json!({
                    "id": id, "name": f["album"].as_str().filter(|a| !a.is_empty()).unwrap_or("Singles"),
                    "artist": f["artist"], "artistId": f["artistId"], "coverArt": f["coverArt"],
                    "songCount": songs.len(), "year": f["year"],
                    "duration": songs.iter().filter_map(|s| s["duration"].as_u64()).sum::<u64>(),
                })
            })
            .collect()
    }
    fn album_songs(&self, id: &str) -> Vec<Value> {
        let mut s: Vec<Value> = self.songs.iter().filter(|s| s["albumId"] == id).cloned().collect();
        s.sort_by_key(|x| x["track"].as_u64().unwrap_or(0));
        s
    }
    fn artists(&self) -> Vec<Value> {
        let mut by: BTreeMap<String, (String, std::collections::BTreeSet<String>)> =
            BTreeMap::new();
        for s in &self.songs {
            let e =
                by.entry(s["artistId"].as_str().unwrap_or("").to_string()).or_insert_with(|| {
                    (s["artist"].as_str().unwrap_or("").to_string(), Default::default())
                });
            e.1.insert(s["albumId"].as_str().unwrap_or("").to_string());
        }
        let mut v: Vec<Value> = by
            .into_iter()
            .map(|(id, (name, albums))| {
                json!({ "id": id, "name": if name.is_empty() { "Unknown artist".into() } else { name }, "albumCount": albums.len() })
            })
            .collect();
        v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        v
    }
}

fn playlists(state: &crate::AppState, lib: &Lib) -> Vec<(Value, Vec<Value>)> {
    let out = fm_app::dispatch("music_playlists", &json!({}), &[], &state.app, &crate::Desktop)
        .map(|o| o.into_bytes())
        .unwrap_or_default();
    let lists: Vec<Value> = serde_json::from_slice(&out).unwrap_or_default();
    lists
        .iter()
        .map(|p| {
            let entries: Vec<Value> = p["songs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|id| lib.songs.iter().find(|s| s["id"] == *id).cloned())
                .collect();
            let head = json!({
                "id": p["id"], "name": p["title"], "songCount": entries.len(), "owner": USER, "public": false,
                "duration": entries.iter().filter_map(|s| s["duration"].as_u64()).sum::<u64>(),
            });
            (head, entries)
        })
        .collect()
}

/// The answer to one method, or a file to send (`Err(reference)`).
fn answer(
    state: &crate::AppState,
    method: &str,
    q: &BTreeMap<String, String>,
) -> Result<Value, String> {
    let id = q.get("id").cloned().unwrap_or_default();
    let ok = |v: Value| Ok(envelope(true, v));
    match method {
        "ping" | "scrobble" | "star" | "unstar" | "setRating" => ok(json!({})),
        "getLicense" => ok(json!({ "license": { "valid": true } })),
        "getOpenSubsonicExtensions" => ok(json!({ "openSubsonicExtensions": [] })),
        "getUser" => ok(
            json!({ "user": { "username": USER, "streamRole": true, "downloadRole": true, "playlistRole": false, "adminRole": false, "folder": [1] } }),
        ),
        "getMusicFolders" => {
            ok(json!({ "musicFolders": { "musicFolder": [{ "id": 1, "name": "Music" }] } }))
        }
        "getArtists" | "getIndexes" => {
            let lib = Lib::read(state);
            let mut index: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for a in lib.artists() {
                let first = a["name"]
                    .as_str()
                    .unwrap_or("#")
                    .chars()
                    .next()
                    .unwrap_or('#')
                    .to_uppercase()
                    .to_string();
                let key = if first.chars().all(char::is_alphabetic) { first } else { "#".into() };
                index.entry(key).or_default().push(a);
            }
            let index: Vec<Value> = index
                .into_iter()
                .map(|(name, artist)| json!({ "name": name, "artist": artist }))
                .collect();
            let key = if method == "getArtists" { "artists" } else { "indexes" };
            ok(json!({ key: { "ignoredArticles": "The", "lastModified": 0, "index": index } }))
        }
        "getArtist" => {
            let lib = Lib::read(state);
            let a = lib
                .artists()
                .into_iter()
                .find(|a| a["id"] == id.as_str())
                .ok_or("no such artist")?;
            let albums: Vec<Value> =
                lib.albums().into_iter().filter(|al| al["artistId"] == id.as_str()).collect();
            let mut a = a;
            a["album"] = json!(albums);
            ok(json!({ "artist": a }))
        }
        "getAlbumList2" | "getAlbumList" => {
            let lib = Lib::read(state);
            let mut albums = lib.albums();
            match q.get("type").map(String::as_str) {
                Some("random") => {
                    albums.sort_by_key(|a| short(&format!("{}{}", a["id"], std::process::id())))
                }
                Some("newest") | Some("recent") => albums.reverse(),
                _ => albums.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str())),
            }
            let size: usize = q.get("size").and_then(|s| s.parse().ok()).unwrap_or(10).min(500);
            let off: usize = q.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);
            let page: Vec<Value> = albums.into_iter().skip(off).take(size).collect();
            let key = if method == "getAlbumList2" { "albumList2" } else { "albumList" };
            ok(json!({ key: { "album": page } }))
        }
        "getAlbum" => {
            let lib = Lib::read(state);
            let mut al =
                lib.albums().into_iter().find(|a| a["id"] == id.as_str()).ok_or("no such album")?;
            al["song"] = json!(lib.album_songs(&id));
            ok(json!({ "album": al }))
        }
        "getSong" => {
            let lib = Lib::read(state);
            let s =
                lib.songs.iter().find(|s| s["id"] == id.as_str()).cloned().ok_or("no such song")?;
            ok(json!({ "song": s }))
        }
        "getRandomSongs" => {
            let lib = Lib::read(state);
            let size: usize = q.get("size").and_then(|s| s.parse().ok()).unwrap_or(10).min(500);
            let salt = format!("{:?}", std::time::SystemTime::now());
            let mut s = lib.songs.clone();
            s.sort_by_key(|x| short(&format!("{}{salt}", x["id"])));
            s.truncate(size);
            ok(json!({ "randomSongs": { "song": s } }))
        }
        "getPlaylists" => {
            let lib = Lib::read(state);
            let p: Vec<Value> = playlists(state, &lib).into_iter().map(|(h, _)| h).collect();
            ok(json!({ "playlists": { "playlist": p } }))
        }
        "getPlaylist" => {
            let lib = Lib::read(state);
            let (mut h, e) = playlists(state, &lib)
                .into_iter()
                .find(|(h, _)| h["id"] == id.as_str())
                .ok_or("no such playlist")?;
            h["entry"] = json!(e);
            ok(json!({ "playlist": h }))
        }
        "search3" | "search2" => {
            let lib = Lib::read(state);
            let words: Vec<String> = q
                .get("query")
                .map(|s| s.trim_matches('"').to_lowercase())
                .unwrap_or_default()
                .split_whitespace()
                .map(String::from)
                .collect();
            let hit = |t: String| words.iter().all(|w| t.to_lowercase().contains(w));
            let n = |k: &str, d: usize| q.get(k).and_then(|s| s.parse().ok()).unwrap_or(d).min(500);
            let artist: Vec<Value> = lib
                .artists()
                .into_iter()
                .filter(|a| hit(a["name"].as_str().unwrap_or("").into()))
                .take(n("artistCount", 20))
                .collect();
            let album: Vec<Value> = lib
                .albums()
                .into_iter()
                .filter(|a| {
                    hit(format!(
                        "{} {}",
                        a["name"].as_str().unwrap_or(""),
                        a["artist"].as_str().unwrap_or("")
                    ))
                })
                .take(n("albumCount", 20))
                .collect();
            let song: Vec<Value> = lib
                .songs
                .iter()
                .filter(|s| {
                    hit(format!(
                        "{} {} {}",
                        s["title"].as_str().unwrap_or(""),
                        s["artist"].as_str().unwrap_or(""),
                        s["album"].as_str().unwrap_or("")
                    ))
                })
                .take(n("songCount", 20))
                .cloned()
                .collect();
            let key = if method == "search3" { "searchResult3" } else { "searchResult2" };
            ok(json!({ key: { "artist": artist, "album": album, "song": song } }))
        }
        _ => Ok(error(0, &format!("{method} is not answered by formicaria"))),
    }
}

/// The file a `stream`, `download` or `getCoverArt` asks for: an asset reference.
fn file_of(state: &crate::AppState, method: &str, id: &str) -> Option<String> {
    if id.starts_with("sha256:") {
        // Only a song's cover or file may be asked for by reference.
        let lib = Lib::read(state);
        return lib.songs.iter().any(|s| s["coverArt"] == id).then(|| id.to_string());
    }
    let out = fm_app::dispatch("music_library", &json!({}), &[], &state.app, &crate::Desktop)
        .ok()?
        .into_bytes();
    let notes: Vec<Value> = serde_json::from_slice(&out).ok()?;
    let pick = |o: &Value| {
        if method == "getCoverArt" {
            Some(text(o, "cover").to_string()).filter(|c| !c.is_empty())
        } else {
            Some(text(o, "audio").to_string()).filter(|a| !a.is_empty())
        }
    };
    if let Some(o) = notes.iter().find(|o| o["id"] == id) {
        return pick(o);
    }
    // An album's cover: its first song's.
    notes
        .iter()
        .map(song)
        .zip(notes.iter())
        .find(|(s, _)| s["albumId"] == id)
        .and_then(|(_, o)| pick(o))
}

/// `/rest/<method>[.view]`. `None`: not a Subsonic request.
pub fn route(
    stream: &mut dyn crate::Conn,
    target: &str,
    body: &[u8],
    range: Option<&str>,
    head: bool,
    state: &crate::AppState,
) -> Option<std::io::Result<()>> {
    let route = target.split('?').next().unwrap_or(target);
    let method = route.strip_prefix("/rest/")?.trim_end_matches(".view").to_string();
    let q = query(target, body);
    let json_wanted = q.get("f").map(String::as_str) == Some("json");
    let send = |stream: &mut dyn crate::Conn, v: Value| {
        if json_wanted {
            crate::write_response(stream, "200 OK", "application/json", v.to_string().as_bytes())
        } else {
            let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
            to_xml("subsonic-response", &v["subsonic-response"], &mut x);
            crate::write_response(stream, "200 OK", "text/xml; charset=utf-8", x.as_bytes())
        }
    };
    let s = settings();
    if !authorized(&q, &s) {
        let why = if s.on {
            "Wrong username or password"
        } else {
            "Subsonic is off in formicaria's Settings"
        };
        return Some(send(stream, error(40, why)));
    }
    if matches!(method.as_str(), "stream" | "download" | "getCoverArt") {
        let id = q.get("id").cloned().unwrap_or_default();
        return Some(match file_of(state, &method, &id) {
            Some(r) => crate::blob::serve(
                stream,
                state,
                &fm_app::Scope::All,
                &r,
                range,
                head,
                false,
                false,
            ),
            None => crate::write_response(stream, "404 Not Found", "text/plain", b"no such file"),
        });
    }
    Some(match answer(state, &method, &q) {
        Ok(v) => send(stream, v),
        Err(e) => send(stream, error(70, &e)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_gives_the_standard_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        // Subsonic's own example: password "sesame", salt "c19b2d".
        assert_eq!(md5_hex(b"sesamec19b2d"), "26719a1196d2a940705a59634eb18eab");
    }

    #[test]
    fn a_player_is_let_in_by_token_or_password_only_while_on() {
        let s = Settings { on: true, password: "sesame".into() };
        let q = |pairs: &[(&str, &str)]| {
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>()
        };
        assert!(authorized(
            &q(&[("u", USER), ("t", "26719a1196d2a940705a59634eb18eab"), ("s", "c19b2d")]),
            &s
        ));
        assert!(authorized(&q(&[("u", USER), ("p", "sesame")]), &s));
        assert!(authorized(&q(&[("u", USER), ("p", "enc:736573616d65")]), &s));
        assert!(!authorized(&q(&[("u", USER), ("p", "wrong")]), &s));
        assert!(!authorized(&q(&[("u", "admin"), ("p", "sesame")]), &s));
        assert!(!authorized(&q(&[("u", USER), ("p", "sesame")]), &Settings { on: false, ..s }));
    }

    #[test]
    fn a_player_sees_the_library_as_music_has_it() {
        let d = tempfile::tempdir().unwrap();
        let vault = d.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg = fm_app::vaults::VaultConfig { name: "v".into(), path: vault, restic: None };
        let state = crate::AppState::new(
            fm_app::App::new(store, vec![cfg], None, false),
            None,
            Vec::new(),
            0,
        );
        let song = |title: &str, track: u32| {
            json!({ "title": title, "body": "", "tags": [], "start": "",
                    "props": { "log": "music", "audio": format!("sha256:{}", "a".repeat(64)), "artist": "The Example Band",
                               "album": "Road", "track": track, "duration_s": 200 } })
        };
        fm_app::dispatch(
            "create_notes",
            &json!({ "notes": [song("Two", 2), song("One", 1)] }),
            &[],
            &state.app,
            &crate::Desktop,
        )
        .unwrap();
        let q = BTreeMap::new();
        let artists = answer(&state, "getArtists", &q).unwrap();
        let a = &artists["subsonic-response"]["artists"]["index"][0]["artist"][0];
        assert_eq!(a["name"], "The Example Band");
        assert_eq!(a["albumCount"], 1);
        let albums = answer(&state, "getAlbumList2", &q).unwrap();
        let al = &albums["subsonic-response"]["albumList2"]["album"][0];
        assert_eq!(
            (al["name"].as_str(), al["songCount"].as_u64(), al["duration"].as_u64()),
            (Some("Road"), Some(2), Some(400))
        );
        let mut q2 = BTreeMap::new();
        q2.insert("id".to_string(), al["id"].as_str().unwrap().to_string());
        let album = answer(&state, "getAlbum", &q2).unwrap();
        let titles: Vec<_> = album["subsonic-response"]["album"]["song"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["title"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(titles, ["One", "Two"], "in track order");
        let mut q3 = BTreeMap::new();
        q3.insert("query".to_string(), "example two".to_string());
        let found = answer(&state, "search3", &q3).unwrap();
        assert_eq!(
            found["subsonic-response"]["searchResult3"]["song"].as_array().unwrap().len(),
            1
        );
        // A stream sends the song's own file; nothing else can be asked for by reference.
        let id = album["subsonic-response"]["album"]["song"][0]["id"].as_str().unwrap().to_string();
        assert_eq!(file_of(&state, "stream", &id), Some(format!("sha256:{}", "a".repeat(64))));
        assert_eq!(file_of(&state, "stream", &format!("sha256:{}", "b".repeat(64))), None);
    }

    #[test]
    fn xml_is_subsonics_shape() {
        let v = envelope(
            true,
            json!({ "album": { "id": "al-1", "name": "Road & Rail", "song": [{ "id": "a" }, { "id": "b" }] } }),
        );
        let mut x = String::new();
        to_xml("subsonic-response", &v["subsonic-response"], &mut x);
        assert!(x.starts_with("<subsonic-response xmlns=\"http://subsonic.org/restapi\""));
        assert!(x.contains(" status=\"ok\""));
        assert!(x.contains(
            "<album id=\"al-1\" name=\"Road &amp; Rail\"><song id=\"a\"/><song id=\"b\"/></album>"
        ));
    }
}

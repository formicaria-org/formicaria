//! **A track saved into a note is a song too** (the owner, 2026-11-21: an album saved *Into the
//! note* showed in the note and nowhere in the player). Pinned through `media::keep`, the one filing
//! step the computer and the phone share: the note gets its attachment, Music gets a song, and both
//! point at the one stored file.

use fm_app::{dispatch, media, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::Path;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn app() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let store = MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: vault.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

fn call(app: &App, cmd: &str, args: Value) -> Value {
    dispatch(cmd, &args, b"", app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
        .unwrap()
}

#[test]
fn a_track_saved_into_a_note_is_also_a_song_with_the_same_file() {
    let (_h, _v, app) = app();
    let made = call(
        &app,
        "create_notes",
        json!({ "notes": [{ "title": "Night Walk", "body": "", "props": {} }] }),
    );
    let note = made[0]["id"].as_str().unwrap().to_string();
    let work = tempdir().unwrap();
    let file = work.path().join("media.mp3");
    std::fs::write(&file, b"ID3\x04\x00\x00\x00\x00\x00\x00 a track, not really one").unwrap();
    let ask: media::Ask = serde_json::from_value(json!({
        "url": "https://t4.bcbits.com/stream/x/mp3-128/1",
        "page": "https://ant-band.bandcamp.com/album/night-walk",
        "what": "mp3",
        "into": "note",
        "note": note,
    }))
    .unwrap();
    let info = json!({ "title": "First light", "artist": "Ant Band", "album": "Night Walk" });
    let opened = media::keep(&app, &NoHost, "", &file, "s1", &ask, &info).unwrap();
    assert_eq!(opened, note, "the sheet's Open still goes to the note");

    let songs = call(&app, "music_library", json!({}));
    let songs = songs.as_array().unwrap();
    assert_eq!(songs.len(), 1, "one song in Music: {songs:?}");
    let song = &songs[0];
    assert_eq!(song["title"], "First light");
    let audio = song["props"]["audio"].as_str().unwrap().to_string();
    assert!(audio.starts_with("sha256:"), "{audio}");

    let body = call(&app, "get", json!({ "id": note }))["body"].as_str().unwrap().to_string();
    assert!(
        body.contains(&format!("(asset:{})", audio.replacen("sha256:", "sha256-", 1))),
        "the note holds the same file: {body}"
    );
}

/// **A note's audio plays and goes into playlists** (2026-11-21): Music lists the notes with sound in
/// them, and turns a note's tracks into songs on request, once, keeping the same file. A picture in
/// the same note is not a track.
#[test]
fn a_notes_audio_is_listed_and_becomes_songs_once() {
    let (_h, vault, app) = app();
    let blobs = fm_core::blob::BlobStore::new(vault.path());
    let song = blobs.put_bytes(b"ID3\x04\x00\x00 a track").unwrap();
    let picture = blobs.put_bytes(b"\x89PNG\r\n\x1a\n a picture").unwrap();
    let hex = |h: &str| h.trim_start_matches("sha256:").to_string();
    let song_hex = hex(&song.hash);
    let pic_hex = hex(&picture.hash);
    let body = format!(
        "An album.\n\n![First light](asset:sha256-{song_hex})\n\n![the cover](asset:sha256-{pic_hex})\n"
    );
    let made = call(
        &app,
        "create_notes",
        json!({ "notes": [{ "title": "Night Walk", "body": body, "props": {} }] }),
    );
    let note = made[0]["id"].as_str().unwrap().to_string();

    let listed = call(&app, "music_notes", json!({}));
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["id"], note.as_str());
    let tracks = listed[0]["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 1, "the picture is not a track: {tracks:?}");
    assert_eq!(tracks[0]["title"], "First light");
    assert!(tracks[0]["song"].is_null());

    let ids = call(&app, "songs_from_note", json!({ "note": note }));
    let ids: Vec<String> = serde_json::from_value(ids).unwrap();
    assert_eq!(ids.len(), 1);
    let songs = call(&app, "music_library", json!({}));
    let songs = songs.as_array().unwrap();
    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0]["id"], ids[0].as_str());
    assert_eq!(songs[0]["props"]["audio"], format!("sha256:{song_hex}"));
    assert_eq!(songs[0]["props"]["album"], "Night Walk");

    // Asked again: the same song, not a second one.
    let again: Vec<String> =
        serde_json::from_value(call(&app, "songs_from_note", json!({ "note": note }))).unwrap();
    assert_eq!(again, ids);
    assert_eq!(call(&app, "music_library", json!({})).as_array().unwrap().len(), 1);
}

/// **A song note holding more tracks than its own** (2026-11-21: tracks saved into the note landed
/// in a song's note): its other tracks are listed, its own file is not listed again.
#[test]
fn a_song_notes_extra_tracks_are_listed() {
    let (_h, vault, app) = app();
    let blobs = fm_core::blob::BlobStore::new(vault.path());
    let own = blobs.put_bytes(b"ID3\x04\x00\x00 its own track").unwrap();
    let extra = blobs.put_bytes(b"ID3\x04\x00\x00 another track").unwrap();
    let hex = |h: &str| h.trim_start_matches("sha256:").to_string();
    let (own_hex, extra_hex) = (hex(&own.hash), hex(&extra.hash));
    let body = format!("![Intro](asset:sha256-{own_hex})\n\n![Second](asset:sha256-{extra_hex})\n");
    call(
        &app,
        "create_notes",
        json!({ "notes": [{ "title": "Intro", "body": body,
                            "props": { "log": "music", "audio": format!("sha256:{own_hex}") } }] }),
    );
    let listed = call(&app, "music_notes", json!({}));
    let tracks = listed[0]["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 1, "{listed:?}");
    assert_eq!(tracks[0]["title"], "Second");
}

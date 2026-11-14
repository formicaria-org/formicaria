//! **An audio file's own tags**, read once at upload (`decisions.md#data`, 2026-11-13, *a sixth
//! group, Music*): title, artist, album, album artist, track, disc, year, genre, duration and the
//! embedded front cover.
//!
//! Read, never written. A blob is content-addressed, so rewriting a tag would change its hash and
//! orphan every note that names it; corrections go to the song's note instead.
//!
//! **Everything here may fail and only degrades the song's details**, never the upload: a file
//! `lofty` cannot parse, or one with no tags, still stores and still plays, and the sheet fills
//! what it can from the filename.

use lofty::config::ParseOptions;
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::Path;

/// What a file says about itself. Every field is optional: a tag that is missing or blank is
/// `None`, never an empty string, so a caller can tell "not said" from "said as nothing".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u16>,
    /// Whole seconds, rounded to the nearest. `None` when the stream's length could not be read.
    pub duration_s: Option<u64>,
    /// The front cover's bytes and MIME, or the first picture when none is marked as the front.
    pub cover: Option<(Vec<u8>, String)>,
}

impl AudioTags {
    /// Did the file say anything at all? An untagged file is still worth its duration.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A cover larger than this is not kept: a tag can carry a full-resolution scan, and a song's
/// picture is drawn small. The song keeps its other details.
pub const MAX_COVER_BYTES: usize = 8 * 1024 * 1024;

/// Read the tags of the audio file at `path`. `None` when it is not an audio file `lofty` knows.
pub fn read_tags(path: &Path) -> Option<AudioTags> {
    // Pictures are read here and then size-checked; `read_properties` is what gives the duration.
    let tagged = Probe::open(path)
        .ok()?
        .options(ParseOptions::new().read_properties(true).read_cover_art(true))
        .guess_file_type()
        .ok()?
        .read()
        .ok()?;
    let mut out = AudioTags::default();
    let secs = tagged.properties().duration().as_millis();
    out.duration_s = (secs > 0).then(|| ((secs + 500) / 1000) as u64);

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Some(out);
    };
    let text = |v: Option<std::borrow::Cow<'_, str>>| {
        v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    out.title = text(tag.title());
    out.artist = text(tag.artist());
    out.album = text(tag.album());
    out.genre = text(tag.genre());
    out.album_artist = tag
        .get_string(ItemKey::AlbumArtist)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    out.track = tag.track().filter(|n| *n > 0);
    out.disc = tag.disk().filter(|n| *n > 0);
    out.year = tag.date().map(|d| d.year).filter(|y| *y > 0);

    let pictures = tag.pictures();
    let front = pictures
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first());
    out.cover =
        front.filter(|p| !p.data().is_empty() && p.data().len() <= MAX_COVER_BYTES).map(|p| {
            let mime = p
                .mime_type()
                .map(|m| m.as_str().to_string())
                .filter(|m| m.starts_with("image/"))
                // A tag that names no type: sniff the bytes, as every other blob is.
                .or_else(|| infer::get(p.data()).map(|t| t.mime_type().to_string()))
                .unwrap_or_else(|| "image/jpeg".to_string());
            (p.data().to_vec(), mime)
        });
    Some(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use lofty::config::WriteOptions;
    use lofty::picture::{MimeType, Picture};
    use lofty::tag::{Tag, TagType};

    /// A second of 8 kHz mono silence as a WAV file — a real, playable audio file written by the
    /// test itself, so no recording is ever committed.
    pub(crate) fn silent_wav(seconds: u32) -> Vec<u8> {
        let rate = 8000u32;
        let data_len = rate * 2 * seconds;
        let mut b = Vec::with_capacity(44 + data_len as usize);
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data_len).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
        b.extend_from_slice(&2u16.to_le_bytes()); // block align
        b.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        b.resize(44 + data_len as usize, 0);
        b
    }

    /// The smallest PNG there is: one transparent pixel.
    pub(crate) const PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    /// Write `silent_wav` to `path` and tag it as an invented song, cover included.
    pub(crate) fn tagged_wav(path: &Path) {
        std::fs::write(path, silent_wav(3)).unwrap();
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("Night Train".into());
        tag.set_artist("The Example Band".into());
        tag.set_album("Invented Songs".into());
        tag.set_genre("Jazz".into());
        tag.set_track(4);
        tag.set_disk(1);
        tag.insert_text(ItemKey::AlbumArtist, "Various Examples".into());
        tag.insert_text(ItemKey::RecordingDate, "2019".into());
        tag.push_picture(
            Picture::unchecked(PIXEL_PNG.to_vec())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::Png)
                .build(),
        );
        tag.save_to_path(path, WriteOptions::default()).unwrap();
    }

    #[test]
    fn a_tagged_file_gives_every_detail_and_its_cover() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        tagged_wav(&path);
        let t = read_tags(&path).expect("a WAV is an audio file");
        assert_eq!(t.title.as_deref(), Some("Night Train"));
        assert_eq!(t.artist.as_deref(), Some("The Example Band"));
        assert_eq!(t.album.as_deref(), Some("Invented Songs"));
        assert_eq!(t.album_artist.as_deref(), Some("Various Examples"));
        assert_eq!(t.genre.as_deref(), Some("Jazz"));
        assert_eq!((t.track, t.disc, t.year), (Some(4), Some(1), Some(2019)));
        assert_eq!(t.duration_s, Some(3));
        let (bytes, mime) = t.cover.expect("the front cover");
        assert_eq!(bytes, PIXEL_PNG);
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn an_untagged_file_still_says_how_long_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.wav");
        std::fs::write(&path, silent_wav(2)).unwrap();
        let t = read_tags(&path).expect("a WAV is an audio file");
        assert_eq!(t.duration_s, Some(2));
        assert_eq!((t.title, t.artist, t.cover), (None, None, None));
    }

    #[test]
    fn a_file_that_is_not_audio_has_no_tags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.txt");
        std::fs::write(&path, "not a song").unwrap();
        assert_eq!(read_tags(&path), None);
    }
}

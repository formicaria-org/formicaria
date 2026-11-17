//! **Sound as MP3, and its tags, without ffmpeg** (`decisions.md` 2026-11-17, *structure, a size
//! budget*). On the phone, Save media fetched the sound with yt-dlp and asked ffmpeg — 35 MB of the
//! APK — for three things: MP3, joining a video's two streams, and the tags. MP3 and the tags are
//! here (the joining is Android's own `MediaMuxer`): the sound decoded by Symphonia, encoded by LAME,
//! tagged by lofty, about a megabyte, and tested like the rest.
//!
//! Pure files in, files out; no network, no vault.

use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::Tag;
use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// The MP3 setting, as Save media names it: `v2` (variable, about 190 kbps) or a constant rate.
fn quality(b: &mut Builder, setting: &str) -> Result<(), String> {
    use mp3lame_encoder::{Bitrate, Quality, VbrMode};
    let e = |e: mp3lame_encoder::BuildError| e.to_string();
    match setting {
        "192" => b.set_brate(Bitrate::Kbps192).map_err(e)?,
        "256" => b.set_brate(Bitrate::Kbps256).map_err(e)?,
        "320" => b.set_brate(Bitrate::Kbps320).map_err(e)?,
        _ => {
            b.set_vbr_mode(VbrMode::Mtrh).map_err(e)?;
            b.set_vbr_quality(Quality::SecondBest).map_err(e)?;
        }
    }
    b.set_quality(Quality::Good).map_err(e)
}

/// **`input`'s sound as an MP3 at `output`.** AAC (in MP4), MP3 or WAV in; one or two channels.
pub fn to_mp3(input: &Path, output: &Path, setting: &str) -> Result<(), String> {
    let file = File::open(input).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = input.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("this sound cannot be read: {e}"))?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("no sound in this file")?.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("this sound's kind cannot be read here: {e}"))?;
    let rate = track.codec_params.sample_rate.ok_or("the sound has no sample rate")?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2).clamp(1, 2);

    let mut b = Builder::new().ok_or("the MP3 encoder could not start")?;
    b.set_num_channels(channels as u8).map_err(|e| e.to_string())?;
    b.set_sample_rate(rate).map_err(|e| e.to_string())?;
    quality(&mut b, setting)?;
    // The header frame (Xing/LAME) players read an MP3's length from, and seek by: reserved first,
    // filled in after the last frame (below).
    b.set_to_write_vbr_tag(true).map_err(|e| e.to_string())?;
    let mut enc = b.build().map_err(|e| e.to_string())?;

    let mut out = File::create(output).map_err(|e| e.to_string())?;
    let mut mp3: Vec<u8> = Vec::new();
    let mut left: Vec<f32> = Vec::new();
    let mut right: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(format!("the sound stopped being readable: {e}")),
        };
        if packet.track_id() != track.id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            // A damaged packet is skipped, as players do.
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        };
        let mut buf: AudioBuffer<f32> = decoded.make_equivalent();
        decoded.convert(&mut buf);
        left.clear();
        right.clear();
        left.extend_from_slice(buf.chan(0));
        if channels == 2 {
            right.extend_from_slice(buf.chan(buf.spec().channels.count().min(2) - 1));
        }
        mp3.clear();
        mp3.reserve(mp3lame_encoder::max_required_buffer_size(left.len()));
        let n = if channels == 2 {
            let pcm: Vec<f32> = left.iter().zip(&right).flat_map(|(l, r)| [*l, *r]).collect();
            enc.encode(InterleavedPcm(&pcm), mp3.spare_capacity_mut())
        } else {
            enc.encode(MonoPcm(&left), mp3.spare_capacity_mut())
        }
        .map_err(|e| e.to_string())?;
        // SAFETY: `encode` wrote `n` bytes into the spare capacity it was given.
        unsafe { mp3.set_len(n) };
        out.write_all(&mp3).map_err(|e| e.to_string())?;
    }
    mp3.clear();
    mp3.reserve(7200);
    let n = enc.flush::<FlushNoGap>(mp3.spare_capacity_mut()).map_err(|e| e.to_string())?;
    // SAFETY: as above.
    unsafe { mp3.set_len(n) };
    out.write_all(&mp3).map_err(|e| e.to_string())?;
    // The header frame, now that the length is known, over the one reserved at the start.
    let mut tag = Vec::with_capacity(enc.lame_tag_size());
    if enc.lame_tag_encode_to_vec(&mut tag).is_some() {
        use std::io::{Seek, SeekFrom};
        out.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        out.write_all(&tag).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// What goes inside the finished file: what any player shows.
#[derive(Debug, Clone, Default)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    /// The cover (a JPEG or PNG), when the media has one.
    pub cover: Option<Vec<u8>>,
}

/// **Write the tags into `path`** (an MP3 or an M4A), before it is stored: a stored file is never
/// rewritten (its name is its hash).
pub fn write_tags(path: &Path, t: &Tags) -> Result<(), String> {
    let mut file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let kind = file.primary_tag_type();
    if file.tag(kind).is_none() {
        file.insert_tag(Tag::new(kind));
    }
    let tag = file.tag_mut(kind).ok_or("no tag")?;
    if let Some(v) = &t.title {
        tag.set_title(v.clone());
    }
    if let Some(v) = &t.artist {
        tag.set_artist(v.clone());
    }
    if let Some(v) = &t.album {
        tag.set_album(v.clone());
    }
    if let Some(y) = t.year {
        tag.set_date(lofty::tag::items::Timestamp { year: y as u16, ..Default::default() });
    }
    if let Some(c) = &t.cover {
        let mime =
            if c.starts_with(&[0x89, b'P', b'N', b'G']) { MimeType::Png } else { MimeType::Jpeg };
        tag.push_picture(
            Picture::unchecked(c.clone()).pic_type(PictureType::CoverFront).mime_type(mime).build(),
        );
    }
    file.save_to_path(path, WriteOptions::default()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tone.m4a")
    }

    #[test]
    fn aac_becomes_an_mp3_of_the_same_length() {
        let d = tempfile::tempdir().unwrap();
        for setting in ["v2", "192", "320"] {
            let out = d.path().join(format!("tone-{setting}.mp3"));
            to_mp3(&fixture(), &out, setting).unwrap();
            let f = lofty::read_from_path(&out).unwrap();
            assert_eq!(f.file_type(), lofty::file::FileType::Mpeg, "{setting}");
            let ms = f.properties().duration().as_millis();
            assert!((900..1200).contains(&ms), "{setting}: {ms} ms");
        }
    }

    #[test]
    fn tags_and_a_cover_go_inside_the_file() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("tone.mp3");
        to_mp3(&fixture(), &out, "v2").unwrap();
        let png = [0x89u8, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 0];
        write_tags(
            &out,
            &Tags {
                title: Some("Leafcutters at night".into()),
                artist: Some("Ant Channel".into()),
                album: Some("Road".into()),
                year: Some(2024),
                cover: Some(png.to_vec()),
            },
        )
        .unwrap();
        let f = lofty::read_from_path(&out).unwrap();
        let t = f.primary_tag().unwrap();
        assert_eq!(t.title().as_deref(), Some("Leafcutters at night"));
        assert_eq!(t.artist().as_deref(), Some("Ant Channel"));
        assert_eq!(t.album().as_deref(), Some("Road"));
        assert_eq!(t.pictures().len(), 1);
        // And an M4A as served takes them too.
        let m4a = d.path().join("tone.m4a");
        std::fs::copy(fixture(), &m4a).unwrap();
        write_tags(&m4a, &Tags { title: Some("Tone".into()), ..Default::default() }).unwrap();
        assert_eq!(
            lofty::read_from_path(&m4a).unwrap().primary_tag().unwrap().title().as_deref(),
            Some("Tone")
        );
    }

    #[test]
    fn a_file_that_is_not_sound_is_said_so() {
        let d = tempfile::tempdir().unwrap();
        let bad = d.path().join("x.m4a");
        std::fs::write(&bad, b"not a sound").unwrap();
        assert!(to_mp3(&bad, &d.path().join("x.mp3"), "v2")
            .unwrap_err()
            .contains("cannot be read"));
    }
}

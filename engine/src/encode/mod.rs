// SPDX-License-Identifier: Apache-2.0

//! Frame sources and the encoders that consume them.

pub mod probe;
pub mod select;

use crate::error::RenderError;
use select::{detect_av1_encoder, detect_h264_encoder};
use std::io::Write;
use std::path::{Path, PathBuf};

pub use select::{EncodeSettings, HW_AV1_CANDIDATES, SW_AV1_CANDIDATES};

/// Legacy alias used in docs/tests.
pub const AV1_CANDIDATES: &[&str] = &[
    "av1_nvenc",
    "av1_qsv",
    "av1_amf",
    "libsvtav1",
    "libaom-av1",
    "librav1e",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeBackend {
    /// Pipe BGRA frames to ffmpeg AV1.
    FfmpegAv1,
    /// Pipe BGRA frames to ffmpeg H.264.
    FfmpegH264,
    /// Write a single raw BGRA dump (tests / no ffmpeg).
    RawDump,
    /// Stream raw BGRA + 16-byte header to stdout.
    StdoutRaw,
    /// Write one PNG per frame into the output directory.
    PngSequence,
}

/// A source of BGRA frames.
///
/// The writer owns the frame buffer and the producer fills it. That is the
/// whole point of this trait: a render of a ten-minute clip at 1080p would
/// otherwise allocate eight megabytes per frame, sixty times a second.
///
/// The producer is called once per frame index, in order.
pub trait FrameSource {
    /// Render frame `index` into `out` as BGRA bytes.
    fn produce(&mut self, index: u64, out: &mut Vec<u8>) -> Result<(), RenderError>;
}

impl<F> FrameSource for F
where
    F: FnMut(u64, &mut Vec<u8>) -> Result<(), RenderError>,
{
    fn produce(&mut self, index: u64, out: &mut Vec<u8>) -> Result<(), RenderError> {
        self(index, out)
    }
}

/// The shape and rate of a frame sequence.
///
/// Width, height and fps always travel together: a frame buffer's length is
/// derived from the first two and the encoder needs the third, so keeping
/// them in one value stops a caller from pairing a buffer with the wrong
/// rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSpec {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl FrameSpec {
    /// The byte length of one BGRA frame.
    pub fn frame_len(&self) -> usize {
        (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4)
    }
}

/// Encode a sequence of BGRA frames by spawning ffmpeg (or raw dump).
pub fn encode_raw_bgra_to_file(
    backend: EncodeBackend,
    settings: &EncodeSettings,
    spec: FrameSpec,
    output: &Path,
    frame_count: u64,
    produce: &mut impl FrameSource,
) -> Result<u64, RenderError> {
    match backend {
        EncodeBackend::RawDump => write_raw_dump(output, spec, frame_count, produce),
        EncodeBackend::StdoutRaw => write_stdout_raw(spec, frame_count, produce),
        EncodeBackend::PngSequence => write_png_sequence(output, spec, frame_count, produce),
        EncodeBackend::FfmpegAv1 => {
            let encoder = match &settings.encoder {
                Some(name) => name.clone(),
                None => detect_av1_encoder(settings.prefer_hw)?,
            };
            write_ffmpeg_pipe(settings, &encoder, spec, output, frame_count, produce, true)
        }
        EncodeBackend::FfmpegH264 => {
            let encoder = match &settings.encoder {
                Some(name) => name.clone(),
                None => detect_h264_encoder(settings.prefer_hw)?,
            };
            write_ffmpeg_pipe(
                settings,
                &encoder,
                spec,
                output,
                frame_count,
                produce,
                false,
            )
        }
    }
}

fn write_raw_dump(
    output: &Path,
    spec: FrameSpec,
    frame_count: u64,
    produce: &mut impl FrameSource,
) -> Result<u64, RenderError> {
    let mut file = std::fs::File::create(output).map_err(|source| RenderError::Io {
        path: output.to_path_buf(),
        source,
    })?;
    let mut buf = vec![0u8; spec.frame_len()];
    let mut n = 0u64;
    while n < frame_count {
        FrameSource::produce(produce, n, &mut buf)?;
        file.write_all(&buf).map_err(|source| RenderError::Io {
            path: output.to_path_buf(),
            source,
        })?;
        n += 1;
    }
    if n == 0 {
        return Err(RenderError::EmptyOutput);
    }
    Ok(n)
}

/// Stream raw BGRA + 16-byte header to stdout. Header: width|height|fps|magic LE u32s;
/// magic `0x4942_5247` == "GBRI".
fn write_stdout_raw(
    spec: FrameSpec,
    frame_count: u64,
    produce: &mut impl FrameSource,
) -> Result<u64, RenderError> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(&spec.width.to_le_bytes());
    header[4..8].copy_from_slice(&spec.height.to_le_bytes());
    header[8..12].copy_from_slice(&spec.fps.to_le_bytes());
    header[12..16].copy_from_slice(&0x4952_4247u32.to_le_bytes());
    let p = PathBuf::from("<stdout>");
    out.write_all(&header).map_err(|source| RenderError::Io {
        path: p.clone(),
        source,
    })?;
    let mut buf = vec![0u8; spec.frame_len()];
    let mut n = 0u64;
    while n < frame_count {
        FrameSource::produce(produce, n, &mut buf)?;
        out.write_all(&buf).map_err(|source| RenderError::Io {
            path: p.clone(),
            source,
        })?;
        n += 1;
    }
    if n == 0 {
        return Err(RenderError::EmptyOutput);
    }
    Ok(n)
}

/// Write one PNG per frame into `output_dir`, naming `frame0001.png` etc.
mod containers;
use containers::{write_ffmpeg_pipe, write_png_sequence};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_nonempty() {
        assert!(!AV1_CANDIDATES.is_empty());
    }

    #[test]
    fn stdout_raw_header_magic_is_gbri() {
        let bytes = 0x4952_4247u32.to_le_bytes();
        assert_eq!(&bytes, b"GBRI");
    }

    #[test]
    fn a_producer_fills_the_buffer_it_is_given() {
        let mut out = Vec::new();
        let mut source = |index: u64, buf: &mut Vec<u8>| {
            buf.clear();
            buf.extend(std::iter::repeat_n(index as u8, 4));
            Ok(())
        };
        source.produce(3, &mut out).expect("frame");
        assert_eq!(out, vec![3, 3, 3, 3]);
    }
}

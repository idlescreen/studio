//! Snapshot baseline compare for `--snapshot-last-only` mode.
//!
//! Two-arg signature: `<current>` bytes vs `<baseline>` bytes, byte-equal.
//! For PNG inputs the comparison decodes both into RGBA8 buffers (since two
//! encoders can produce different IDAT zlib output for identical pixels, but
//! always decode to the same RGBA8). For raw BGRA inputs we memcmp the bytes
//! directly — that path catches renderer-determinism regressions.

use std::io::Read;
use std::path::Path;

/// Snapshot mismatch reason (carried back through [`compare`] for diagnostics).
#[derive(Debug)]
pub enum SnapshotMismatch {
    MissingBaseline(String),
    LengthMismatch {
        baseline: usize,
        current: usize,
    },
    PixelMismatch {
        byte_offset: usize,
        baseline: u8,
        current: u8,
    },
    Io {
        path: String,
        source: std::io::Error,
    },
}

impl std::fmt::Display for SnapshotMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBaseline(p) => {
                write!(f, "baseline missing at {p} (run with --update-baselines to seed)")
            }
            Self::LengthMismatch { baseline, current } => {
                write!(f, "baseline length {baseline} != current length {current}")
            }
            Self::PixelMismatch {
                byte_offset,
                baseline,
                current,
            } => write!(
                f,
                "pixel mismatch at byte {byte_offset}: baseline {baseline:#04x} vs current {current:#04x}"
            ),
            Self::Io { path, source } => write!(f, "snapshot io error on {path}: {source}"),
        }
    }
}

impl std::error::Error for SnapshotMismatch {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Compare two files. PNG inputs are decoded into RGBA8 (deterministic across
/// zlib settings); raw `.bgra` inputs are memcmp'd verbatim.
pub fn compare(current_path: &Path, baseline_path: &Path) -> Result<(), SnapshotMismatch> {
    if !baseline_path.is_file() {
        return Err(SnapshotMismatch::MissingBaseline(
            baseline_path.display().to_string(),
        ));
    }
    let ext = baseline_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let current = read_bytes(current_path).map_err(|source| SnapshotMismatch::Io {
        path: current_path.display().to_string(),
        source,
    })?;
    let baseline = read_bytes(baseline_path).map_err(|source| SnapshotMismatch::Io {
        path: baseline_path.display().to_string(),
        source,
    })?;
    if ext == "png" {
        let cur_decoded = decode_png_rgba(&current, current_path)?;
        let base_decoded = decode_png_rgba(&baseline, baseline_path)?;
        if cur_decoded.len() != base_decoded.len() {
            return Err(SnapshotMismatch::LengthMismatch {
                baseline: base_decoded.len(),
                current: cur_decoded.len(),
            });
        }
        for (i, (c, b)) in cur_decoded.iter().zip(base_decoded.iter()).enumerate() {
            if c != b {
                return Err(SnapshotMismatch::PixelMismatch {
                    byte_offset: i,
                    baseline: *b,
                    current: *c,
                });
            }
        }
        return Ok(());
    }
    // Raw: memcmp.
    if current.len() != baseline.len() {
        return Err(SnapshotMismatch::LengthMismatch {
            baseline: baseline.len(),
            current: current.len(),
        });
    }
    for (i, (c, b)) in current.iter().zip(baseline.iter()).enumerate() {
        if c != b {
            return Err(SnapshotMismatch::PixelMismatch {
                byte_offset: i,
                baseline: *b,
                current: *c,
            });
        }
    }
    Ok(())
}

/// Write `bytes` to `path`, creating parents as needed.
pub fn write_baseline(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
}

/// Read all bytes from `path`.
pub fn read_baseline(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    let mut f = std::fs::File::open(path)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

fn decode_png_rgba(bytes: &[u8], path: &Path) -> Result<Vec<u8>, SnapshotMismatch> {
    let (_w, _h, rgba) = crate::png::decode_rgba8(bytes).map_err(|e| SnapshotMismatch::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tempfile replacement: unique dir under std::env::temp_dir, removed on Drop.
    struct TmpDir(std::path::PathBuf);
    impl TmpDir {
        fn new() -> Self {
            static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "idle-snap-{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("tmp");
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn byte_equal_passes() {
        let tmp = TmpDir::new();
        let a = tmp.path().join("a.bgra");
        let b = tmp.path().join("b.bgra");
        std::fs::write(&a, [1u8, 2, 3, 4]).expect("write");
        std::fs::write(&b, [1u8, 2, 3, 4]).expect("write");
        compare(&a, &b).expect("eq");
    }

    #[test]
    fn byte_unequal_fails() {
        let tmp = TmpDir::new();
        let a = tmp.path().join("a.bgra");
        let b = tmp.path().join("b.bgra");
        std::fs::write(&a, [1u8, 2, 3, 4]).expect("write");
        std::fs::write(&b, [1u8, 2, 3, 5]).expect("write");
        let e = compare(&a, &b).expect_err("diff");
        assert!(matches!(e, SnapshotMismatch::PixelMismatch { .. }));
    }

    #[test]
    fn missing_baseline_surfaces() {
        let tmp = TmpDir::new();
        let a = tmp.path().join("a.bgra");
        let b = tmp.path().join("absent.bgra");
        std::fs::write(&a, [1u8]).expect("write");
        let e = compare(&a, &b).expect_err("missing");
        assert!(matches!(e, SnapshotMismatch::MissingBaseline(_)));
    }

    #[test]
    fn png_compare_decodes_to_rgba() {
        let tmp = TmpDir::new();
        let a = tmp.path().join("a.png");
        let b = tmp.path().join("b.png");
        let bgra = vec![10u8, 20, 30, 255];
        let png_a = crate::png_writer::encode_bgra_frame_to_png(1, 1, &bgra);
        std::fs::write(&a, &png_a).expect("write");
        std::fs::write(&b, &png_a).expect("write");
        compare(&a, &b).expect("png eq");
    }
}

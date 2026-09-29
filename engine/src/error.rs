use std::path::PathBuf;

/// Fallible render outcomes.
#[derive(Debug)]
pub enum RenderError {
    Duration(String),
    Job(String),
    Plugin(String),
    Raster(String),
    FfmpegMissing,
    Ffmpeg(String),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    EmptyOutput,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duration(m) => write!(f, "invalid duration: {m}"),
            Self::Job(m) => write!(f, "invalid job: {m}"),
            Self::Plugin(m) => write!(f, "plugin load failed: {m}"),
            Self::Raster(m) => write!(f, "font/raster unavailable: {m}"),
            Self::FfmpegMissing => write!(f, "ffmpeg not found on PATH"),
            Self::Ffmpeg(m) => write!(f, "ffmpeg failed: {m}"),
            Self::Io { path, source } => write!(f, "io error on {}: {source}", path.display()),
            Self::EmptyOutput => write!(f, "encode cancelled or wrote zero frames"),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

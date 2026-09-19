use std::path::PathBuf;

#[derive(Debug)]
pub enum StudioError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json(String),
    Render(String),
    RenderMissing,
    Queue(String),
}

impl std::fmt::Display for StudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error on {}: {source}", path.display()),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Render(m) => write!(f, "render failed: {m}"),
            Self::RenderMissing => {
                write!(
                    f,
                    "render binary not found (set RENDER / IDLE_RENDER or PATH)"
                )
            }
            Self::Queue(m) => write!(f, "queue error: {m}"),
        }
    }
}

impl std::error::Error for StudioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<idle_render::json::Error> for StudioError {
    fn from(e: idle_render::json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

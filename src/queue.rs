use crate::error::StudioError;
use crate::job::StudioJob;
use idle_render::json::Value;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl JobStatus {
    /// serde `snake_case` wire name.
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown job status '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub job: StudioJob,
    pub status: JobStatus,
    pub message: String,
}

impl QueueEntry {
    fn to_value(&self) -> Value {
        Value::Object(vec![
            ("job".into(), self.job.to_value()),
            ("status".into(), Value::Str(self.status.as_str().into())),
            ("message".into(), Value::Str(self.message.clone())),
        ])
    }
    fn from_value(v: &Value) -> Result<Self, String> {
        let job = v
            .get("job")
            .ok_or_else(|| "missing field `job`".to_string())
            .and_then(StudioJob::from_value)?;
        let status = match v.get("status") {
            Some(Value::Str(s)) => JobStatus::from_str(s)?,
            Some(_) => return Err("invalid type for `status`: expected string".into()),
            None => return Err("missing field `status`".into()),
        };
        let message = match v.get("message") {
            None | Some(Value::Null) => String::new(),
            Some(Value::Str(s)) => s.clone(),
            Some(_) => return Err("invalid type for `message`: expected string".into()),
        };
        Ok(QueueEntry {
            job,
            status,
            message,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobQueue {
    pub entries: Vec<QueueEntry>,
}

impl JobQueue {
    fn to_value(&self) -> Value {
        Value::Object(vec![(
            "entries".into(),
            Value::Array(self.entries.iter().map(QueueEntry::to_value).collect()),
        )])
    }
    fn from_value(v: &Value) -> Result<Self, String> {
        let entries = match v.get("entries") {
            Some(Value::Array(items)) => items
                .iter()
                .map(QueueEntry::from_value)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("invalid type for `entries`: expected array".into()),
            None => return Err("missing field `entries`".into()),
        };
        Ok(JobQueue { entries })
    }
}

impl JobQueue {
    /// Strict load: parse errors surface as `StudioError::Json`.
    pub fn load(path: &Path) -> Result<Self, StudioError> {
        use std::io::Read;
        // O_NOFOLLOW refuses symlinks — the queue file is owned by the
        // operator and must not be a redirect to attacker-controlled content
        // (JSON poisoning / deserialization DoS).
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|source| StudioError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .map_err(|source| StudioError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let v = idle_render::json::parse(&raw).map_err(|e| StudioError::Json(e.to_string()))?;
        Self::from_value(&v).map_err(StudioError::Json)
    }

    /// Load, but recover from a corrupt queue: the unparseable file is moved
    /// aside to `<path>.corrupt-<unix_ts>` (jobs preserved for manual rescue)
    /// and an empty queue is returned so the TUI still opens. IO failures
    /// (permissions, symlink) still propagate.
    pub fn load_or_recover(path: &Path) -> Result<Self, StudioError> {
        match Self::load(path) {
            Err(StudioError::Json(e)) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let corrupt = path.with_extension(format!("json.corrupt-{ts}"));
                if let Err(source) = fs::rename(path, &corrupt) {
                    return Err(StudioError::Io {
                        path: path.to_path_buf(),
                        source,
                    });
                }
                eprintln!(
                    "idle-studio: corrupt queue moved to {} ({e})",
                    corrupt.display()
                );
                Ok(Self::default())
            }
            other => other,
        }
    }

    /// Atomic save: tmp file + fsync + rename. A crash mid-write leaves the
    /// previous queue intact; rename replaces (never follows) a symlink at
    /// `path`, so the save side matches the load side's symlink policy.
    pub fn save(&self, path: &Path) -> Result<(), StudioError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| StudioError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let raw = self.to_value().to_json_pretty();
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&tmp)
                .map_err(|source| StudioError::Io {
                    path: tmp.clone(),
                    source,
                })?;
            file.write_all(raw.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|source| StudioError::Io {
                    path: tmp.clone(),
                    source,
                })?;
        }
        fs::rename(&tmp, path).map_err(|source| StudioError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn enqueue(&mut self, job: StudioJob) {
        self.entries.push(QueueEntry {
            job,
            status: JobStatus::Pending,
            message: String::new(),
        });
    }

    /// First unused `job-N` id — `len()+1` collides after deletions and
    /// duplicate ids share the same `/tmp/idle-studio-<id>.json` job file.
    pub fn next_id(&self) -> String {
        let mut n = self.entries.len() + 1;
        while self.entries.iter().any(|e| e.job.id == format!("job-{n}")) {
            n += 1;
        }
        format!("job-{n}")
    }

    pub fn next_pending_index(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.status == JobStatus::Pending)
    }
}

/// Default queue path under the user's config dir or CWD.
pub fn default_queue_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("idle-studio").join("queue.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("idle-studio")
            .join("queue.json");
    }
    PathBuf::from("idle-studio-queue.json")
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;

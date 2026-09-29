//! Studio queue entry = render [`JobSpec`] + id.
//!
//! Studio does not reimplement export. It stores jobs and asks `render` to run them.

use idle_render::json::Value;
use idle_render::JobSpec;
use std::path::PathBuf;

/// One queued item for the Director UI.
#[derive(Debug, Clone, PartialEq)]
pub struct StudioJob {
    pub id: String,
    /// Serialized flat (serde `flatten`): `id` first, then JobSpec fields.
    pub spec: JobSpec,
}

impl StudioJob {
    pub fn new(id: String, spec: JobSpec) -> Self {
        Self { id, spec }
    }

    /// JSON object: `id` followed by the JobSpec fields (was serde flatten).
    pub fn to_value(&self) -> Value {
        let mut pairs = vec![("id".to_string(), Value::Str(self.id.clone()))];
        if let Value::Object(spec_pairs) = self.spec.to_value() {
            pairs.extend(spec_pairs);
        }
        Value::Object(pairs)
    }

    /// Inverse of [`StudioJob::to_value`]; JobSpec ignores the `id` key.
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let id = match v.get("id") {
            Some(Value::Str(s)) => s.clone(),
            Some(_) => return Err("invalid type for `id`: expected string".into()),
            None => return Err("missing field `id`".into()),
        };
        Ok(Self {
            id,
            spec: JobSpec::from_value(v)?,
        })
    }

    /// Write a temp job-file and return its path (caller deletes).
    ///
    /// The filename is sanitized (`id` is user-supplied via `--id` — raw
    /// interpolation allowed `../` traversal out of the temp dir) and made
    /// unique with pid+counter so duplicate ids can't share one file.
    pub fn write_job_file(&self) -> Result<PathBuf, String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);

        let safe_id: String = self
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let safe_id = if safe_id.is_empty() || safe_id.bytes().all(|b| b == b'.' || b == b'_') {
            "job".to_string()
        } else {
            safe_id
        };
        let path = std::env::temp_dir().join(format!(
            "idle-studio-{}-{safe_id}-{}.json",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        self.spec.save_path(&path).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> JobSpec {
        JobSpec {
            effect: "ripple".into(),
            plugin_path: None,
            seed: 1,
            fps: 30,
            duration: "10s".into(),
            output: PathBuf::from("/tmp/o.mkv"),
            width: 1280,
            height: 720,
            cols: None,
            rows: None,
            dry_run: false,
            raw: false,
            segment: Some("5s".into()),
            audio: None,
            resume: false,
            crf: 35,
            preset: None,
            encoder: None,
            prefer_hw: true,
            gpu_upscale: true,
            format: None,
            container: None,
            baseline_dir: None,
            snapshot_last_only: false,
            update_baselines: false,
            cpu_raster: false,
        }
    }

    #[test]
    fn flatten_roundtrip() {
        let j = StudioJob {
            id: "1".into(),
            spec: spec(),
        };
        let s = j.to_value().to_json();
        let v = idle_render::json::parse(&s).expect("parse");
        let back = StudioJob::from_value(&v).expect("de");
        assert_eq!(back.id, "1");
        assert_eq!(back.spec.effect, "ripple");
        assert_eq!(back.spec.segment.as_deref(), Some("5s"));
    }

    #[test]
    fn hostile_ids_cannot_escape_temp_dir() {
        for id in [
            "../../etc/evil",
            "..",
            "..\\win",
            "a/b/c",
            "",
            "...",
            "x\0y",
        ] {
            let j = StudioJob::new(id.into(), spec());
            let p = j.write_job_file().unwrap_or_else(|e| panic!("{id:?}: {e}"));
            assert_eq!(p.parent().unwrap(), std::env::temp_dir(), "id {id:?}");
            let name = p.file_name().unwrap().to_string_lossy();
            assert!(!name.contains(".."), "id {id:?} -> {name}");
            let _ = std::fs::remove_file(&p);
        }
    }

    #[test]
    fn duplicate_ids_get_distinct_files() {
        let j = StudioJob::new("dup".into(), spec());
        let a = j.write_job_file().unwrap();
        let b = j.write_job_file().unwrap();
        assert_ne!(a, b);
        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);
    }
}

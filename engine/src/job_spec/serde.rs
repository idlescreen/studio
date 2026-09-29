// SPDX-License-Identifier: Apache-2.0

use super::*;

impl JobSpec {
    pub fn to_value(&self) -> Value {
        fn opt_str(o: &Option<impl AsRef<str>>) -> Value {
            match o {
                Some(s) => Value::Str(s.as_ref().to_string()),
                None => Value::Null,
            }
        }
        fn opt_path(o: &Option<PathBuf>) -> Value {
            match o {
                Some(p) => Value::Str(p.display().to_string()),
                None => Value::Null,
            }
        }
        Value::Object(vec![
            ("effect".into(), Value::Str(self.effect.clone())),
            ("plugin_path".into(), opt_path(&self.plugin_path)),
            ("seed".into(), Value::UInt(self.seed)),
            ("fps".into(), Value::UInt(self.fps as u64)),
            ("duration".into(), Value::Str(self.duration.clone())),
            (
                "output".into(),
                Value::Str(self.output.display().to_string()),
            ),
            ("width".into(), Value::UInt(self.width as u64)),
            ("height".into(), Value::UInt(self.height as u64)),
            (
                "cols".into(),
                self.cols.map_or(Value::Null, |v| Value::UInt(v as u64)),
            ),
            (
                "rows".into(),
                self.rows.map_or(Value::Null, |v| Value::UInt(v as u64)),
            ),
            ("dry_run".into(), Value::Bool(self.dry_run)),
            ("raw".into(), Value::Bool(self.raw)),
            ("segment".into(), opt_str(&self.segment)),
            ("audio".into(), opt_path(&self.audio)),
            ("resume".into(), Value::Bool(self.resume)),
            ("crf".into(), Value::UInt(self.crf as u64)),
            ("preset".into(), opt_str(&self.preset)),
            ("encoder".into(), opt_str(&self.encoder)),
            ("prefer_hw".into(), Value::Bool(self.prefer_hw)),
            ("gpu_upscale".into(), Value::Bool(self.gpu_upscale)),
            ("format".into(), opt_str(&self.format)),
            ("container".into(), opt_str(&self.container)),
            ("baseline_dir".into(), opt_path(&self.baseline_dir)),
            (
                "snapshot_last_only".into(),
                Value::Bool(self.snapshot_last_only),
            ),
            (
                "update_baselines".into(),
                Value::Bool(self.update_baselines),
            ),
            ("cpu_raster".into(), Value::Bool(self.cpu_raster)),
        ])
    }

    /// Inverse of [`JobSpec::to_value`]; also used for `StudioJob`'s
    /// serde-flatten (unknown keys — e.g. `id` — are ignored).
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let obj = v
            .as_object()
            .ok_or_else(|| "invalid type: expected object".to_string())?;
        fn field<'a>(obj: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
            obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        }
        fn opt_str(obj: &[(String, Value)], key: &str) -> Result<Option<String>, String> {
            match field(obj, key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Str(s)) => Ok(Some(s.clone())),
                Some(_) => Err(format!("invalid type for `{key}`: expected string")),
            }
        }
        fn opt_path(obj: &[(String, Value)], key: &str) -> Result<Option<PathBuf>, String> {
            Ok(opt_str(obj, key)?.map(PathBuf::from))
        }
        fn opt_usize(obj: &[(String, Value)], key: &str) -> Result<Option<usize>, String> {
            match field(obj, key) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v
                    .as_u64()
                    .map(|u| Some(u as usize))
                    .ok_or_else(|| format!("invalid type for `{key}`: expected unsigned integer")),
            }
        }
        fn req_str(obj: &[(String, Value)], key: &str) -> Result<String, String> {
            match field(obj, key) {
                None => Err(format!("missing field `{key}`")),
                Some(Value::Str(s)) => Ok(s.clone()),
                Some(_) => Err(format!("invalid type for `{key}`: expected string")),
            }
        }
        fn num<T>(obj: &[(String, Value)], key: &str, dflt: T) -> Result<T, String>
        where
            T: TryFrom<u64>,
        {
            match field(obj, key) {
                None => Ok(dflt),
                Some(v) => v
                    .as_u64()
                    .and_then(|u| T::try_from(u).ok())
                    .ok_or_else(|| format!("invalid type for `{key}`: expected unsigned integer")),
            }
        }
        fn boolean(obj: &[(String, Value)], key: &str, dflt: bool) -> Result<bool, String> {
            match field(obj, key) {
                None => Ok(dflt),
                Some(v) => v
                    .as_bool()
                    .ok_or_else(|| format!("invalid type for `{key}`: expected bool")),
            }
        }
        Ok(JobSpec {
            effect: req_str(obj, "effect")?,
            plugin_path: opt_path(obj, "plugin_path")?,
            seed: num(obj, "seed", default_seed())?,
            fps: num(obj, "fps", default_fps())?,
            duration: req_str(obj, "duration")?,
            output: PathBuf::from(req_str(obj, "output")?),
            width: num(obj, "width", default_w())?,
            height: num(obj, "height", default_h())?,
            cols: opt_usize(obj, "cols")?,
            rows: opt_usize(obj, "rows")?,
            dry_run: boolean(obj, "dry_run", false)?,
            raw: boolean(obj, "raw", false)?,
            segment: opt_str(obj, "segment")?,
            audio: opt_path(obj, "audio")?,
            resume: boolean(obj, "resume", false)?,
            crf: num(obj, "crf", default_crf())?,
            preset: opt_str(obj, "preset")?,
            encoder: opt_str(obj, "encoder")?,
            prefer_hw: boolean(obj, "prefer_hw", true)?,
            gpu_upscale: boolean(obj, "gpu_upscale", true)?,
            format: opt_str(obj, "format")?,
            container: opt_str(obj, "container")?,
            baseline_dir: opt_path(obj, "baseline_dir")?,
            snapshot_last_only: boolean(obj, "snapshot_last_only", false)?,
            update_baselines: boolean(obj, "update_baselines", false)?,
            cpu_raster: boolean(obj, "cpu_raster", false)?,
        })
    }

    pub fn save_path(&self, path: &Path) -> Result<(), RenderError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| RenderError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let raw = self.to_json();
        // Atomic: tmp + rename so a crash can't leave a half-written job file
        // for `render --job-file` to choke on.
        let tmp = path.with_extension("job.tmp");
        std::fs::write(&tmp, raw).map_err(|source| RenderError::Io {
            path: tmp.clone(),
            source,
        })?;
        std::fs::rename(&tmp, path).map_err(|source| RenderError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

use idle_render::JobSpec;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// The scene names the daemon ships.
///
/// The list is read from the daemon rather than kept here: a name the
/// Director offers that the daemon cannot render is worse than a missing
/// entry. It is fetched once per form, not once per frame.
pub fn effects() -> Vec<&'static str> {
    idle_render::effect_names()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Queue,
    NewJob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Effect,
    Text,
    Duration,
    Width,
    Height,
    Fps,
    Crf,
    Output,
    DryRun,
}

pub const FORM_FIELDS: &[FormField] = &[
    FormField::Effect,
    FormField::Text,
    FormField::Duration,
    FormField::Width,
    FormField::Height,
    FormField::Fps,
    FormField::Crf,
    FormField::Output,
    FormField::DryRun,
];

pub struct NewJobForm {
    pub field_i: usize,
    pub effect_i: usize,
    /// The daemon's scene names, read once when the form is built.
    pub effects: Vec<&'static str>,
    pub text: String,
    pub duration: String,
    pub width: String,
    pub height: String,
    pub fps: String,
    pub crf: String,
    pub output: String,
    pub dry_run: bool,
    /// When true, typing edits the focused text field.
    pub editing: bool,
}

impl Default for NewJobForm {
    fn default() -> Self {
        let effects = effects();
        let effect = effects.first().copied().unwrap_or("beams");
        Self {
            field_i: 0,
            effect_i: 0,
            effects,
            text: idle_render::scene::DEFAULT_TEXT.to_string(),
            duration: "10s".into(),
            width: "1280".into(),
            height: "720".into(),
            fps: "30".into(),
            crf: "35".into(),
            output: default_output_path(effect),
            dry_run: false,
            editing: false,
        }
    }
}

impl NewJobForm {
    pub fn field(&self) -> FormField {
        FORM_FIELDS[self.field_i % FORM_FIELDS.len()]
    }

    pub fn effect(&self) -> &'static str {
        self.effects
            .get(self.effect_i % self.effects.len().max(1))
            .copied()
            .unwrap_or("beams")
    }

    pub fn cycle_effect(&mut self, dir: i32) {
        let n = self.effects.len() as i32;
        if n == 0 {
            return;
        }
        let i = self.effect_i as i32 + dir;
        self.effect_i = ((i % n) + n) as usize % self.effects.len();
        // Refresh default output stem when effect changes (if still on pattern).
        self.output = default_output_path(self.effect());
    }

    pub fn next_field(&mut self, dir: i32) {
        let n = FORM_FIELDS.len() as i32;
        let i = self.field_i as i32 + dir;
        self.field_i = ((i % n) + n) as usize % FORM_FIELDS.len();
        self.editing = false;
    }

    pub fn text_mut(&mut self) -> Option<&mut String> {
        match self.field() {
            FormField::Text => Some(&mut self.text),
            FormField::Duration => Some(&mut self.duration),
            FormField::Width => Some(&mut self.width),
            FormField::Height => Some(&mut self.height),
            FormField::Fps => Some(&mut self.fps),
            FormField::Crf => Some(&mut self.crf),
            FormField::Output => Some(&mut self.output),
            FormField::Effect | FormField::DryRun => None,
        }
    }

    pub fn to_job_spec(&self) -> Result<JobSpec, String> {
        let width: u32 = self
            .width
            .trim()
            .parse()
            .map_err(|_| "width must be a number".to_string())?;
        let height: u32 = self
            .height
            .trim()
            .parse()
            .map_err(|_| "height must be a number".to_string())?;
        let fps: u32 = self
            .fps
            .trim()
            .parse()
            .map_err(|_| "fps must be a number".to_string())?;
        let crf: u8 = self
            .crf
            .trim()
            .parse()
            .map_err(|_| "crf must be 0–63".to_string())?;
        if width == 0 || height == 0 || fps == 0 {
            return Err("width, height, and fps must be > 0".into());
        }
        let output = PathBuf::from(self.output.trim());
        if output.as_os_str().is_empty() {
            return Err("output path required".into());
        }
        let text = self.text.trim();
        let text = if text.is_empty() {
            idle_render::scene::DEFAULT_TEXT.to_string()
        } else {
            text.to_string()
        };
        Ok(JobSpec {
            effect: self.effect().to_string(),
            text,
            plugin_path: None,
            seed: 0x00C0_FFEE,
            fps,
            duration: self.duration.trim().to_string(),
            output,
            width,
            height,
            cols: None,
            rows: None,
            dry_run: self.dry_run,
            raw: false,
            segment: None,
            audio: None,
            resume: false,
            crf,
            preset: None,
            encoder: None,
            prefer_hw: true,
            gpu_upscale: false, // match host: CPU upscale path for predictable export
            format: None,
            container: None,
            baseline_dir: None,
            snapshot_last_only: false,
            update_baselines: false,
            cpu_raster: false,
        })
    }
}

pub fn default_output_path(effect: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = std::env::var("HOME")
        .map(|h| format!("{h}/Videos"))
        .unwrap_or_else(|_| "/tmp".into());
    format!("{dir}/idlescreen-{effect}-{ts}.mkv")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_form_has_official_effect() {
        let f = NewJobForm::default();
        assert!(f.effects.contains(&f.effect()));
        assert!(f.output.contains("idlescreen-"));
        assert!(f.output.ends_with(".mkv"));
    }

    #[test]
    fn the_effect_list_is_the_daemons_own() {
        let f = NewJobForm::default();
        assert_eq!(f.effects.len(), 53);
        assert!(f.effects.contains(&"beams"));
        assert!(f.effects.contains(&"thunderstorm"));
        // Names the daemon does not ship must not be offered.
        assert!(!f.effects.contains(&"storm"));
        assert!(!f.effects.contains(&"ascii"));
    }

    #[test]
    fn cycle_effect_wraps() {
        let mut f = NewJobForm::default();
        for _ in 0..f.effects.len() + 2 {
            f.cycle_effect(1);
        }
        assert!(f.effects.contains(&f.effect()));
    }

    #[test]
    fn to_job_spec_parses() {
        let f = NewJobForm::default();
        let s = f.to_job_spec().expect("spec");
        assert_eq!(s.effect, "aurora");
        assert_eq!(s.width, 1280);
        assert_eq!(s.fps, 30);
        assert_eq!(s.text, "IDLESCREEN");
    }

    #[test]
    fn an_empty_text_falls_back_to_the_daemon_default() {
        let f = NewJobForm {
            text: "   ".into(),
            ..NewJobForm::default()
        };
        let s = f.to_job_spec().expect("spec");
        assert_eq!(s.text, "IDLESCREEN");
    }
}

//! Property tests for parse/plan protocol logic.
//!
//! Deterministic xorshift64 generators replace `proptest`: fixed seeds keep
//! failures reproducible; coverage matches the old proptest cases.

#[cfg(test)]
mod rng {
    /// xorshift64* — deterministic, seed-stamped in test names on failure.
    pub struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        /// Uniform in `lo..=hi` (hi >= lo).
        pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
            lo + self.next() % (hi - lo + 1)
        }
    }
}

#[cfg(test)]
mod duration_props {
    use super::rng::Rng;
    use crate::duration::parse_duration_secs;
    use std::time::Duration;

    #[test]
    fn seconds_suffix_roundtrip() {
        let mut rng = Rng(0xD0A7_10C5_0001);
        for _ in 0..64 {
            let n = rng.range(1, 10_000);
            let d = parse_duration_secs(&format!("{n}s")).expect("parse");
            assert_eq!(d, Duration::from_secs(n));
        }
    }

    #[test]
    fn bare_number_is_seconds() {
        let mut rng = Rng(0xD0A7_10C5_0002);
        for _ in 0..64 {
            let n = rng.range(1, 10_000);
            let d = parse_duration_secs(&n.to_string()).expect("parse");
            assert_eq!(d, Duration::from_secs(n));
        }
    }

    #[test]
    fn zero_always_errors() {
        for unit in ["", "s", "m", "h", "d"] {
            let raw = if unit.is_empty() {
                "0".to_string()
            } else {
                format!("0{unit}")
            };
            assert!(parse_duration_secs(&raw).is_err());
        }
    }
}

#[cfg(test)]
mod segment_props {
    use super::rng::Rng;
    use crate::models::RenderJob;
    use crate::segment::plan_segments;
    use std::path::PathBuf;
    use std::time::Duration;

    fn job(total: u64, seg: u64) -> RenderJob {
        RenderJob {
            effect: "beams".into(),
            plugin_path: None,
            seed: 1,
            fps: 30,
            duration: Duration::from_secs(total.max(1)),
            width: 64,
            height: 64,
            output: PathBuf::from("/tmp/master.mkv"),
            cols: None,
            rows: None,
            dry_run: true,
            segment: Some(Duration::from_secs(seg.max(1))),
            audio: None,
            resume: false,
            crf: 35,
            preset: None,
            encoder: None,
            prefer_hw: true,
            gpu_upscale: true,
            format: crate::models::OutputFormat::Mp4,
            container: crate::models::Container::Mkv,
            baseline_dir: None,
            snapshot_last_only: false,
            update_baselines: false,
            cpu_raster: false,
        }
    }

    #[test]
    fn plan_len_matches_segment_count() {
        let mut rng = Rng(0x5E66_E07A_0003);
        for _ in 0..48 {
            let total = rng.range(1, 50_000);
            let seg = rng.range(1, 10_000);
            let j = job(total, seg);
            let plans = plan_segments(&j).expect("plan");
            assert_eq!(plans.len() as u64, j.segment_count());
            let sum: u64 = plans.iter().map(|p| p.duration.as_secs()).sum();
            // saturating segments may overshoot last part only by covering total
            assert!(sum >= j.duration.as_secs() || plans.len() == 1);
        }
    }
}

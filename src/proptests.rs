#[cfg(test)]
mod job_props {
    use crate::job::StudioJob;
    use idle_render::JobSpec;
    use std::path::PathBuf;

    /// xorshift64* — deterministic replacement for proptest's generators.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn range(&mut self, lo: u64, hi: u64) -> u64 {
            lo + self.next() % (hi - lo + 1)
        }
        /// Random `[a-z]` string, 3–12 chars.
        fn ident(&mut self) -> String {
            let n = self.range(3, 12);
            (0..n)
                .map(|_| (b'a' + (self.next() % 26) as u8) as char)
                .collect()
        }
    }

    #[test]
    fn job_file_roundtrips_effect() {
        let mut rng = Rng(0xB17C_0DE5_0001);
        for _ in 0..32 {
            let effect = rng.ident();
            let seed = rng.next();
            let fps = rng.range(1, 120) as u32;
            let j = StudioJob {
                id: "t".into(),
                spec: JobSpec {
                    effect: effect.clone(),
                    plugin_path: None,
                    seed,
                    fps,
                    duration: "10s".into(),
                    output: PathBuf::from("/tmp/out.mkv"),
                    width: 1280,
                    height: 720,
                    cols: None,
                    rows: None,
                    dry_run: true,
                    raw: false,
                    segment: None,
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
                },
            };
            let path = j.write_job_file().expect("write");
            let loaded = JobSpec::load_path(&path).expect("load");
            let _ = std::fs::remove_file(&path);
            assert_eq!(loaded.effect, effect);
            assert!(loaded.dry_run);
        }
    }
}

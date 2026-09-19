//! Verify the in-repo PNG decoder against committed baseline fixtures.
//! (Kept as a permanent regression test for the hand-rolled inflate path.)

#[test]
fn committed_baselines_decode() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/baselines");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "png").unwrap_or(false) {
            let bytes = std::fs::read(&p).unwrap();
            let (w, h, rgba) = idle_render::png::decode_rgba8(&bytes)
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert_eq!(rgba.len(), (w * h * 4) as usize);
            assert!(w > 0 && h > 0);
            n += 1;
        }
    }
    assert_eq!(n, 5, "expected 5 committed png baselines");
}

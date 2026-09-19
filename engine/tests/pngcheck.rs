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
}

#[test]
fn decoder_never_panics_on_malformed_input() {
    use idle_render::png::{decode_rgba8, encode_rgba8};
    let png = encode_rgba8(4, 4, &[7u8; 64]);
    // Every truncation — Err or Ok, never panic.
    for len in 0..png.len() {
        let _ = decode_rgba8(&png[..len]);
    }
    // Deterministic byte-flips across the whole file (hits chunk walk,
    // CRC path, zlib header, and inflate tables).
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..512 {
        let mut m = png.clone();
        for _ in 0..rng.range(1, 4) {
            let i = rng.range(0, (m.len() - 1) as u64) as usize;
            m[i] ^= ((rng.next() & 0xFF) as u8) | 1;
        }
        let _ = decode_rgba8(&m);
    }
    // Pure garbage and pathological chunk sizes.
    for len in 0..64usize {
        let mut g = vec![0u8; len];
        for b in g.iter_mut() {
            *b = (rng.next() & 0xFF) as u8;
        }
        let _ = decode_rgba8(&g);
    }
    // IHDR claims huge dimensions — must not allocate unbounded output.
    let mut huge = png.clone();
    huge[16..20].copy_from_slice(&u32::MAX.to_be_bytes()); // width
    huge[20..24].copy_from_slice(&u32::MAX.to_be_bytes()); // height
    let _ = decode_rgba8(&huge);
}

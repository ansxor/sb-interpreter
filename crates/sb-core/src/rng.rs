//! TinyMT32 random-number generator (M1-T9) — the engine behind `RND`/`RNDF`/`RANDOMIZE`.
//!
//! SmileBASIC keeps **8 independent TinyMT32 series** (seed IDs 0–7), a 16-byte state each,
//! in a per-series table (disasm `@0x1d08000`). Every draw advances the series and tempers
//! out a 32-bit word. Bit-exactness here is what makes a seeded program reproducible — the
//! `otya_test.sb3` asserts (`RANDOMIZE I,1` then `RND(I,100)` → 89,33,33,52,…66,
//! `RNDF(I)` ≈ 0.836095) are the conformance fixture.
//!
//! ## Algorithm (TinyMT32, default parameters)
//!
//! The reference is the canonical TinyMT32 (`tinymt32.c`, 3-clause BSD); SmileBASIC uses the
//! standard default parameters `mat1=0x8f7011ee mat2=0xfc78ff1f tmat=0x3793fdff`. The init
//! seeds `status = [seed, mat1, mat2, tmat]`, mixes 7 rounds with the
//! `1812433253`-multiplier recurrence, certifies the period, then runs 8 pre-loop
//! `next_state` steps. Each generate = one `next_state` then `temper`.
//!
//! ## RND / RNDF construction (disassembled)
//!
//! - **RND(max)**: one raw 32-bit draw reduced to `0..max-1` by an unsigned modulo
//!   (`raw % max`; reduce helper disasm `@0x1fd4e8` — a software modulo, **no** rejection
//!   redraw, so it is plain `raw % max`). `RND(1)` always draws then yields 0.
//! - **RNDF()**: a 53-bit double from **two** draws (the "double-draw" — two `next` calls):
//!   `a = draw1 >> 5` (27 bits), `b = draw2 >> 6` (26 bits), result `(a*2^26 + b) * 2^-53`
//!   in `[0, 1)` — matches RNDF core disasm `@0x1eac60` (`lsr #5`/`lsr #6`, const `2^26`
//!   `0x4190…`, const `2^-53` `0x3CA0…`).
//!
//! See `spec/instructions/{rnd,rndf,randomize}.yaml`.

/// Canonical TinyMT32 default parameters (the set SmileBASIC ships).
const MAT1: u32 = 0x8f70_11ee;
const MAT2: u32 = 0xfc78_ff1f;
const TMAT: u32 = 0x3793_fdff;
const MASK: u32 = 0x7fff_ffff;
const MIN_LOOP: u32 = 8;
const PRE_LOOP: u32 = 8;

/// One TinyMT32 series: a 4-word (127-bit) internal state.
#[derive(Debug, Clone, Copy)]
struct TinyMt32 {
    status: [u32; 4],
}

impl TinyMt32 {
    /// Initialise from a 32-bit seed (`tinymt32_init`): seed the state, mix, certify the
    /// period, then run `PRE_LOOP` `next_state` steps. SmileBASIC's `RANDOMIZE seed_id,
    /// seed_value` does exactly this (disasm seed path `@0x26f580`) — no extra discard
    /// draw (unlike osb's `random.d`, which adds a `popFront`; that would shift the whole
    /// sequence and break the `otya_test` golden, so we don't).
    fn new(seed: u32) -> Self {
        let mut status = [seed, MAT1, MAT2, TMAT];
        for i in 1..MIN_LOOP as usize {
            let prev = status[(i - 1) & 3];
            status[i & 3] ^=
                (i as u32).wrapping_add(1_812_433_253u32.wrapping_mul(prev ^ (prev >> 30)));
        }
        let mut t = TinyMt32 { status };
        t.period_certification();
        for _ in 0..PRE_LOOP {
            t.next_state();
        }
        t
    }

    /// Guard against the all-zero state (which would lock the generator at zero): replace
    /// it with the ASCII `"TINY"` constants. Matches `period_certification`.
    fn period_certification(&mut self) {
        if (self.status[0] & MASK) == 0
            && self.status[1] == 0
            && self.status[2] == 0
            && self.status[3] == 0
        {
            self.status = [b'T' as u32, b'I' as u32, b'N' as u32, b'Y' as u32];
        }
    }

    /// Advance the state one step (`tinymt32_next_state`).
    fn next_state(&mut self) {
        let mut y = self.status[3];
        let mut x = (self.status[0] & MASK) ^ self.status[1] ^ self.status[2];
        x ^= x << 1;
        y ^= (y >> 1) ^ x;
        self.status[0] = self.status[1];
        self.status[1] = self.status[2];
        self.status[2] = x ^ (y << 10);
        self.status[3] = y;
        // `-(y & 1)` masks in mat1/mat2 when the low bit is set.
        let mask = 0u32.wrapping_sub(y & 1);
        self.status[1] ^= mask & MAT1;
        self.status[2] ^= mask & MAT2;
    }

    /// Temper the current state into a 32-bit output (`tinymt32_temper`). Note the **`+`**
    /// (not `^`) combining `status[0]` with `status[2] >> 8` — SmileBASIC ships the
    /// non-`LINEARITY_CHECK` build.
    fn temper(&self) -> u32 {
        let t0 = self.status[3];
        let t1 = self.status[0].wrapping_add(self.status[2] >> 8);
        let mut out = t0 ^ t1;
        let mask = 0u32.wrapping_sub(t1 & 1);
        out ^= mask & TMAT;
        out
    }

    /// One 32-bit draw (`tinymt32_generate_uint32`): advance then temper.
    fn next_u32(&mut self) -> u32 {
        self.next_state();
        self.temper()
    }
}

/// The eight TinyMT32 series SmileBASIC exposes through seed IDs 0–7.
#[derive(Debug, Clone)]
pub struct Rng {
    series: [TinyMt32; 8],
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    /// Build the eight series. On real hardware each is entropy-seeded at startup, so RND
    /// output is non-deterministic until a `RANDOMIZE seed_id, seed_value` with a non-zero
    /// `seed_value`. `sb-core` has no I/O/entropy (it must build for wasm32 and stay
    /// deterministic), so each series gets a fixed, distinct fallback seed; a program that
    /// relies on un-seeded RND is non-reproducible on real SB anyway. The real entropy
    /// source is oracle-queued (`HARVEST_QUEUE.md`).
    pub fn new() -> Self {
        // Distinct, deterministic fallback seeds (series index + 1).
        let series = std::array::from_fn(|i| TinyMt32::new(i as u32 + 1));
        Rng { series }
    }

    /// `RND(seed_id, max)`: draw one 32-bit word from `series[seed_id]` and reduce to
    /// `0..max-1` by unsigned modulo. Always advances the series (even for `max <= 1`).
    /// `max <= 0` yields 0 (degenerate range; the caller has already validated `max >= 0`,
    /// and `max == 0` only reaches here as a 0-length range — oracle-queued).
    pub fn rnd(&mut self, seed_id: usize, max: i32) -> i32 {
        let raw = self.series[seed_id].next_u32();
        if max <= 0 {
            0
        } else {
            (raw % max as u32) as i32
        }
    }

    /// `RNDF(seed_id)`: a 53-bit double in `[0, 1)` from two draws of `series[seed_id]`.
    pub fn rndf(&mut self, seed_id: usize) -> f64 {
        let a = (self.series[seed_id].next_u32() >> 5) as f64; // 27 bits
        let b = (self.series[seed_id].next_u32() >> 6) as f64; // 26 bits
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0) // *2^26, then *2^-53
    }

    /// `RANDOMIZE seed_id, seed_value`: reseed one series. A non-zero `seed_value` gives a
    /// reproducible sequence; `seed_value == 0` requests entropy (deterministic fallback
    /// here, as in [`Rng::new`]).
    pub fn randomize(&mut self, seed_id: usize, seed_value: i32) {
        let seed = if seed_value == 0 {
            seed_id as u32 + 1 // deterministic entropy fallback
        } else {
            seed_value as u32
        };
        self.series[seed_id] = TinyMt32::new(seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `otya_test.sb3` golden: after `RANDOMIZE series, 1`, the first four `RND(100)`
    /// draws are 89, 33, 33, 52; then `RNDF` ≈ 0.836095; then `RND(100)` == 66. Captured
    /// from real SmileBASIC, so this pins the whole TinyMT path bit-for-bit.
    #[test]
    fn otya_seeded_sequence_is_bit_exact() {
        let mut rng = Rng::new();
        rng.randomize(0, 1);
        assert_eq!(rng.rnd(0, 100), 89);
        assert_eq!(rng.rnd(0, 100), 33);
        assert_eq!(rng.rnd(0, 100), 33);
        assert_eq!(rng.rnd(0, 100), 52);
        let f = rng.rndf(0);
        assert_eq!(format!("{:.6}", f), "0.836095");
        assert_eq!(rng.rnd(0, 100), 66);
    }

    /// Every series with the same non-zero seed produces the same sequence (the series
    /// index does not perturb a seeded series — only which state cell is used).
    #[test]
    fn seeded_series_are_independent_but_identical_for_equal_seed() {
        for s in 0..8 {
            let mut rng = Rng::new();
            rng.randomize(s, 1);
            assert_eq!(rng.rnd(s, 100), 89, "series {s}");
        }
    }

    /// `RNDF` lands in `[0, 1)`.
    #[test]
    fn rndf_in_unit_interval() {
        let mut rng = Rng::new();
        rng.randomize(3, 12345);
        for _ in 0..1000 {
            let f = rng.rndf(3);
            assert!((0.0..1.0).contains(&f), "rndf out of range: {f}");
        }
    }

    /// `RND(1)` is always 0 but still advances the series (a following draw differs from a
    /// fresh series' first draw).
    #[test]
    fn rnd_one_is_zero_and_consumes_a_draw() {
        let mut rng = Rng::new();
        rng.randomize(0, 1);
        assert_eq!(rng.rnd(0, 1), 0);
        // Having consumed one draw, the next RND(100) is the *second* of the golden run.
        assert_eq!(rng.rnd(0, 100), 33);
    }

    /// `RANDOMIZE` re-seeding restarts the sequence.
    #[test]
    fn reseeding_restarts_the_sequence() {
        let mut rng = Rng::new();
        rng.randomize(0, 1);
        assert_eq!(rng.rnd(0, 100), 89);
        rng.randomize(0, 1);
        assert_eq!(rng.rnd(0, 100), 89);
    }
}

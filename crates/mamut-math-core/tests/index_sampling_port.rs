//! MATH-E2E-001 T2a — index-sampling port contract (`RngPort::next_index`).
//!
//! This integration test pins the **native** index-sampling behavior of the
//! MAMUT RNG port. It is deliberately separate from the legacy consumer
//! adapter parity (a PCG32-backed `ChaCha20Rng` override, T2b): here only the
//! native `MamutRng` stream and the object-safe default of
//! [`mamut_math_core::RngPort::next_index`] are checked.
//!
//! Evidence classes used below:
//! - **Pre-port baseline capture** — native indices observed through the public
//!   `MamutRng::next_u64` stream *before* the port method existed, using the
//!   documented sampler transcribed in this file.
//! - **Crafted words** — synthetic raw `u64` draws with hand-computed results
//!   and hand-computed rejection thresholds (`2^64 % n`), so rejection,
//!   no-rejection, and boundary buckets are asserted from first principles
//!   rather than from the implementation under test.
//! - **Regression pin** — native stream equivalence (values and exact raw draw
//!   counts) that fails loudly on any accidental sampler drift.
//!
//! Synthetic unit probes only: no engineering case, no new dependency.
//!
//! Bounds that do not fit a 32-bit `usize` — and the pinned 64-bit values
//! derived from them — are gated to `target_pointer_width = "64"`, so this file
//! still compiles on a 32-bit target. No 32-bit runner or toolchain was
//! executed for this evidence, and none is claimed.

use core::num::NonZeroUsize;

use mamut_math_core::{
    MamutRng, RngMetadata, RngPort, Seed, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION,
};

/// Width-independent bounds: every bound and pinned index below fits in a
/// 32-bit `usize` (`<= 65_537`).
const NATIVE_BOUNDS: [usize; 20] = [
    1, 2, 3, 4, 5, 6, 7, 8, 10, 16, 17, 31, 32, 100, 255, 256, 257, 1000, 65_536, 65_537,
];

/// Native baseline captured **before** this port existed: sequential
/// `(bound, index)` pairs drawn from one `MamutRng` root seeded with
/// `Seed::from_u64(42)` through the public `next_u64` stream and the documented
/// multiply-high-with-rejection sampler.
///
/// Width-independent: these are the first 20 draws of that stream, so the
/// values are identical on any pointer width.
const NATIVE_SEED42_BASELINE: [(usize, usize); 20] = [
    (1, 0),
    (2, 0),
    (3, 2),
    (4, 2),
    (5, 3),
    (6, 5),
    (7, 2),
    (8, 0),
    (10, 1),
    (16, 8),
    (17, 9),
    (31, 0),
    (32, 13),
    (100, 69),
    (255, 15),
    (256, 24),
    (257, 250),
    (1000, 661),
    (65_536, 35_516),
    (65_537, 36_552),
];

/// 64-bit-only continuation of the same seed-42 stream: bounds above `u32` and
/// the indices derived from them.
#[cfg(target_pointer_width = "64")]
const WIDE_BOUNDS: [usize; 4] = [1 << 32, (1 << 32) + 1, 1 << 63, usize::MAX];

#[cfg(target_pointer_width = "64")]
const WIDE_SEED42_BASELINE: [(usize, usize); 4] = [
    (1 << 32, 4_083_466_999),
    ((1 << 32) + 1, 4_063_713_034),
    (1 << 63, 104_303_150_596_057_830),
    (usize::MAX, 1_143_817_501_250_967_854),
];

/// One pinned table: the bounds and their expected `(bound, index)` values.
type Seed42Table = (&'static [usize], &'static [(usize, usize)]);

/// `(bounds, pinned values)` pairs consumed in order over one continuous
/// seed-42 stream. A 64-bit target appends the wide continuation; a 32-bit
/// target runs only the width-independent table.
#[cfg(target_pointer_width = "64")]
const SEED42_TABLES: &[Seed42Table] = &[
    (&NATIVE_BOUNDS, &NATIVE_SEED42_BASELINE),
    (&WIDE_BOUNDS, &WIDE_SEED42_BASELINE),
];

#[cfg(not(target_pointer_width = "64"))]
const SEED42_TABLES: &[Seed42Table] = &[(&NATIVE_BOUNDS, &NATIVE_SEED42_BASELINE)];

/// Build a non-zero upper bound (the port's domain).
fn upper(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("test bound is non-zero")
}

/// Call the port method through `&mut dyn RngPort`, proving object safety.
fn default_index(rng: &mut dyn RngPort, n: usize) -> usize {
    rng.next_index(upper(n))
}

/// Test-side transcription of the documented index sampler (multiply-high with
/// rejection), used as the reference for the native stream. It consumes raw
/// `u64` draws through the public [`RngPort::next_u64`] only.
fn reference_next_index(rng: &mut dyn RngPort, n: usize) -> usize {
    assert!(n >= 1, "reference_next_index requires n >= 1");
    let n = n as u64;
    let mut x = rng.next_u64();
    let mut m = (x as u128) * (n as u128);
    let mut low = m as u64;
    if low < n {
        // 2^64 % n, computed without overflow.
        let threshold = n.wrapping_neg() % n;
        while low < threshold {
            x = rng.next_u64();
            m = (x as u128) * (n as u128);
            low = m as u64;
        }
    }
    (m >> 64) as usize
}

/// Deterministic synthetic raw words for the foreign-port range sweep.
fn crafted_words(count: usize) -> Vec<u64> {
    (0..count as u64)
        .map(|k| k.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDEAD_BEEF_CAFE_F00D)
        .collect()
}

/// A minimal `RngPort` backed by a fixed word queue.
///
/// It deliberately does **not** override `next_index`: this is the
/// existing-implementer compatibility case (a port written against the
/// pre-port trait still compiles and receives the default sampler).
struct WordPort {
    words: Vec<u64>,
    pos: usize,
    word_draws: usize,
}

impl WordPort {
    fn new(words: Vec<u64>) -> Self {
        Self {
            words,
            pos: 0,
            word_draws: 0,
        }
    }

    fn word_draws(&self) -> usize {
        self.word_draws
    }
}

impl RngPort for WordPort {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        assert!(
            self.pos < self.words.len(),
            "crafted raw draw budget exceeded"
        );
        let value = self.words[self.pos];
        self.pos += 1;
        self.word_draws += 1;
        value
    }

    fn next_unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for chunk in dst.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            for (slot, byte) in chunk.iter_mut().zip(bytes) {
                *slot = byte;
            }
        }
    }

    fn metadata(&self) -> RngMetadata {
        RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            [0u8; 8],
            Vec::new(),
            0,
        )
        .expect("frozen algorithm id is valid")
    }

    fn derive_child_port(&self, _stream_index: u64) -> Box<dyn RngPort> {
        Box::new(WordPort::new(self.words.clone()))
    }
}

/// Forwarding port that counts raw `u64` draws.
///
/// Like [`WordPort`] it does not override `next_index`, so the default body is
/// exercised through its own `next_u64` (the ordinary forwarder pattern).
struct CountingPort<P: RngPort> {
    inner: P,
    word_draws: usize,
}

impl<P: RngPort> CountingPort<P> {
    fn new(inner: P) -> Self {
        Self {
            inner,
            word_draws: 0,
        }
    }

    fn word_draws(&self) -> usize {
        self.word_draws
    }
}

impl<P: RngPort> RngPort for CountingPort<P> {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.word_draws += 1;
        self.inner.next_u64()
    }

    fn next_unit_f64(&mut self) -> f64 {
        self.inner.next_unit_f64()
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        self.inner.fill_bytes(dst);
    }

    fn metadata(&self) -> RngMetadata {
        self.inner.metadata()
    }

    fn derive_child_port(&self, stream_index: u64) -> Box<dyn RngPort> {
        self.inner.derive_child_port(stream_index)
    }
}

/// `upper = 1` must still consume its historical single raw draw: the sampler
/// is not short-circuited for the trivial bound, so consumers that count draws
/// keep an unchanged stream position.
#[test]
fn upper_one_consumes_exactly_one_raw_draw() {
    let mut rng = WordPort::new(vec![0x1111_2222_3333_4444, 0x5555_6666_7777_8888]);
    assert_eq!(default_index(&mut rng, 1), 0);
    assert_eq!(rng.word_draws(), 1);
    assert_eq!(
        rng.next_u64(),
        0x5555_6666_7777_8888,
        "the upper=1 draw must still be consumed"
    );

    let mut rng = WordPort::new(vec![u64::MAX, 7]);
    assert_eq!(default_index(&mut rng, 1), 0);
    assert_eq!(rng.word_draws(), 1);
    assert_eq!(rng.next_u64(), 7);
}

/// Crafted rejection cases with hand-computed thresholds `2^64 % n`.
#[test]
fn crafted_rejection_consumes_extra_raw_draws() {
    // n = 3: threshold 2^64 % 3 = 1. x = 0 gives low = 0 < 1 -> reject;
    // x = 1 gives m = 3, low = 3 >= 1 -> accept with high word 0.
    let mut rng = WordPort::new(vec![0, 1]);
    assert_eq!(default_index(&mut rng, 3), 0);
    assert_eq!(rng.word_draws(), 2);

    // n = 6: threshold 2^64 % 6 = 4. x = 0 -> low 0 (reject);
    // x = 2^63 -> m = 3*2^64 exactly, low 0 (reject);
    // x = 2^63 + 1 -> m = 3*2^64 + 6, low 6 >= 4 (accept), high word 3.
    let mut rng = WordPort::new(vec![0, 1 << 63, (1 << 63) + 1]);
    assert_eq!(default_index(&mut rng, 6), 3);
    assert_eq!(rng.word_draws(), 3);

    // n = 10: threshold 2^64 % 10 = 6. x = 0 -> low 0 (reject);
    // x = 2^63 + 1 -> m = 5*2^64 + 10, low 10 >= 6 (accept), high word 5.
    let mut rng = WordPort::new(vec![0, (1 << 63) + 1]);
    assert_eq!(default_index(&mut rng, 10), 5);
    assert_eq!(rng.word_draws(), 2);
}

/// Power-of-two bounds divide `2^64`, so nothing is ever rejected and the
/// index is a plain high-word shift of the single raw draw.
#[test]
fn power_of_two_bounds_never_reject() {
    let mut rng = WordPort::new(vec![0, 1 << 63, u64::MAX]);
    assert_eq!(default_index(&mut rng, 2), 0);
    assert_eq!(default_index(&mut rng, 2), 1);
    assert_eq!(default_index(&mut rng, 2), 1);
    assert_eq!(rng.word_draws(), 3);
}

/// 64-bit-only power-of-two bounds `2^32` and `2^63` (both need a 64-bit
/// `usize`). Raw words are `u64` on every target.
#[cfg(target_pointer_width = "64")]
#[test]
fn power_of_two_wide_bounds_never_reject() {
    let mut rng = WordPort::new(vec![0x1234_5678_9ABC_DEF0]);
    assert_eq!(default_index(&mut rng, 1 << 32), 0x1234_5678);
    assert_eq!(rng.word_draws(), 1);

    let mut rng = WordPort::new(vec![u64::MAX]);
    assert_eq!(default_index(&mut rng, 1 << 63), 0x7FFF_FFFF_FFFF_FFFF);
    assert_eq!(rng.word_draws(), 1);
}

/// Non-power bounds: the top bucket is reachable, and the `2^64 - 1` width
/// boundary only rejects the all-zero word.
#[test]
fn nonpower_bounds_map_crafted_words_to_expected_buckets() {
    // n = 1000: threshold 2^64 % 1000 = 616.
    // x = 2^64 - 1 -> low = 2^64 - 1000 >= 616, high word 999.
    let mut rng = WordPort::new(vec![u64::MAX]);
    assert_eq!(default_index(&mut rng, 1000), 999);
    assert_eq!(rng.word_draws(), 1);

    // x = 2^63 -> low 0 < 616 (reject); then x = 2^64 - 1 accepts.
    let mut rng = WordPort::new(vec![1 << 63, u64::MAX]);
    assert_eq!(default_index(&mut rng, 1000), 999);
    assert_eq!(rng.word_draws(), 2);

    // x = 0 -> low 0 < 616 (reject); x = 1 -> low 1000 >= 616, high word 0.
    let mut rng = WordPort::new(vec![0, 1]);
    assert_eq!(default_index(&mut rng, 1000), 0);
    assert_eq!(rng.word_draws(), 2);

    // n = u32::MAX (a boundary that exists on every pointer width):
    // threshold 2^64 % (2^32-1) = 1, so only x = 0 rejects; x = 1 yields 0.
    let mut rng = WordPort::new(vec![0, 1]);
    assert_eq!(default_index(&mut rng, u32::MAX as usize), 0);
    assert_eq!(rng.word_draws(), 2);
}

/// 64-bit-only width boundary: `n = usize::MAX = 2^64 - 1`, threshold
/// `2^64 % (2^64-1) = 1`, so only `x = 0` rejects; `x = 1` yields `0`.
#[cfg(target_pointer_width = "64")]
#[test]
fn usize_max_bound_rejects_only_zero_word() {
    let mut rng = WordPort::new(vec![0, 1]);
    assert_eq!(default_index(&mut rng, usize::MAX), 0);
    assert_eq!(rng.word_draws(), 2);
}

/// Run the pinned tables in order over one continuous native seed-42 stream,
/// asserting values, in-range results, reference equivalence and identical
/// cumulative raw draw counts.
fn assert_seed42_pins(tables: &[Seed42Table]) {
    let mut default_port = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));
    let mut reference_port = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));

    for (bounds, values) in tables {
        assert_eq!(bounds.len(), values.len(), "bound/value table out of sync");
        for (&bound, &(n, expected)) in bounds.iter().zip(values.iter()) {
            assert_eq!(n, bound, "baseline and bound table out of sync");
            let observed = default_index(&mut default_port, n);
            assert_eq!(
                observed, expected,
                "native default index drift for bound {n}"
            );
            assert!(observed < n, "index {observed} out of range for bound {n}");

            let reference = reference_next_index(&mut reference_port, n);
            assert_eq!(
                observed, reference,
                "default diverges from the documented reference for bound {n}"
            );
            assert_eq!(
                default_port.word_draws(),
                reference_port.word_draws(),
                "raw draw count drift for bound {n}"
            );
        }
    }
}

/// The native default reproduces the pre-port baseline values, matches the
/// independent reference on the same raw stream, and consumes the exact same
/// number of raw `u64` draws.
#[test]
fn native_default_matches_pinned_baseline_and_reference_draw_counts() {
    assert_seed42_pins(SEED42_TABLES);
}

/// Range sweep over a native stream: every call returns in `0..upper`.
#[test]
fn native_default_stays_in_range_for_many_bounds() {
    let mut port = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));
    for n in 1..=257usize {
        for _ in 0..4 {
            assert!(default_index(&mut port, n) < n, "out of range for {n}");
        }
    }
}

/// 64-bit-only range sweep for bounds above `u32`.
#[cfg(target_pointer_width = "64")]
#[test]
fn native_default_stays_in_range_for_wide_bounds() {
    let mut port = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));
    for n in [1 << 32, (1 << 32) + 1, 1 << 63, (1 << 63) + 1, usize::MAX] {
        for _ in 0..4 {
            assert!(default_index(&mut port, n) < n, "out of range for {n}");
        }
    }
}

/// A foreign port that implements only the pre-port methods still samples
/// correctly through the default, consuming exactly one draw per bounded call
/// in the no-rejection regime.
#[test]
fn foreign_port_without_override_uses_default_sampler() {
    for n in 1..=64usize {
        let mut port = WordPort::new(crafted_words(8));
        for _ in 0..4 {
            assert!(default_index(&mut port, n) < n, "out of range for {n}");
        }
        assert_eq!(
            port.word_draws(),
            4,
            "crafted words must not hit the rejection branch at bound {n}"
        );
    }
}

//! MATH-E2E-001 M2 — IEEE raw environmental-selection contract.
//!
//! Proves the raw-`f64` route
//! (`mamut_nsga3::ieee_selection::select_environmental`) shares **one**
//! arithmetic body with the checked selection chain
//! (`nondom_sort` -> `normalize` -> `associate`/`perp_distance` -> niche
//! select): identical finite outputs, identical structural error kinds,
//! deterministic ordering and zero RNG consumption, with the single intended
//! difference that unsanitised `NaN`/`±Inf` fitness entries are accepted
//! instead of rejected at the finiteness gates.
//!
//! Expected values are independently derived synthetic pins (documented inline)
//! or the pre-existing checked public route; they are never produced by the new
//! raw implementation, and no second legacy algorithm body is copied here.
//! These synthetic controls do **not** establish engineering-request
//! reachability or final numerical `MATH_VERIFY`.

use core::num::NonZeroUsize;

use mamut_math_core::{
    ErrorKind, MathError, RngMetadata, RngPort, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION,
};
use mamut_nsga3::{ieee_selection, select_environmental, uniform_reference_points};

/// Counting `RngPort`: every draw is recorded. The selection chain is
/// documented as RNG-free, so every case below must record zero draws.
#[derive(Default)]
struct CountingPort {
    draws: usize,
}

impl CountingPort {
    fn new() -> Self {
        Self { draws: 0 }
    }
}

impl RngPort for CountingPort {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.draws += 1;
        0
    }

    fn next_unit_f64(&mut self) -> f64 {
        self.draws += 1;
        0.5
    }

    fn next_index(&mut self, _upper_exclusive: NonZeroUsize) -> usize {
        self.draws += 1;
        0
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for byte in dst.iter_mut() {
            *byte = 0;
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
        Box::new(CountingPort::new())
    }
}

/// Checked public route under test.
fn checked(
    fitnesses: &[Vec<f64>],
    n: usize,
    refs: &[Vec<f64>],
    rng: &mut CountingPort,
) -> Result<Vec<usize>, MathError> {
    select_environmental(fitnesses, n, refs, rng)
}

/// New raw IEEE route under test.
fn raw(
    fitnesses: &[Vec<f64>],
    n: usize,
    refs: &[Vec<f64>],
    rng: &mut CountingPort,
) -> Result<Vec<usize>, MathError> {
    ieee_selection::select_environmental(fitnesses, n, refs, rng)
}

fn assert_kind<T>(result: Result<T, MathError>, kind: ErrorKind) {
    assert_eq!(result.err().expect("expected a typed error").kind(), kind);
}

/// A caller-owned 5-objective matrix (the frozen synthetic corpus shape).
fn synthetic_fitnesses(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n.max(1) as f64;
            vec![
                1.0 + t,
                1.0 + (1.0 - t),
                1.0 + 0.5 * t,
                1.0 + 0.5 * (1.0 - t),
                1.0 + 0.3 * t,
            ]
        })
        .collect()
}

/// Default finite corpus: raw and checked return the identical index sequence
/// and neither consumes an RNG draw.
#[test]
fn finite_checked_and_raw_agree_with_zero_draws() {
    let fitnesses = synthetic_fitnesses(20);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut crng = CountingPort::new();
    let mut rrng = CountingPort::new();
    let c = checked(&fitnesses, 10, &refs, &mut crng).unwrap();
    let r = raw(&fitnesses, 10, &refs, &mut rrng).unwrap();
    assert_eq!(c, r);
    assert_eq!(c.len(), 10);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

/// Independently derived finite pin (committed base `ab4d8ed`). Rows
/// `[0,3] [1,2] [2,1] [3,0]` are all rank-1, so with `n = 3` the splitting
/// front is the whole population. Derivation: ideal `[0,0]`, ASF extremes
/// `[3,0]`/`[0,3]`, intercepts `[3,3]`, normalized `[0,1] [1/3,2/3]
/// [2/3,1/3] [1,0]`, refs `[0,1]`/`[1,0]`, associations `(0,0) (0,d) (1,d)
/// (1,0)`. Counts `[0,0]` pick index `0`, the minimum count then forces `3`,
/// and the middle index tie-break selects `1`, giving `[0, 3, 1]`; no draws.
#[test]
fn pinned_finite_selection_matches_derived_indices() {
    let fitnesses = vec![
        vec![0.0, 3.0],
        vec![1.0, 2.0],
        vec![2.0, 1.0],
        vec![3.0, 0.0],
    ];
    let refs = uniform_reference_points(2, 1).unwrap();

    let mut crng = CountingPort::new();
    assert_eq!(
        checked(&fitnesses, 3, &refs, &mut crng).unwrap(),
        vec![0, 3, 1]
    );
    let mut rrng = CountingPort::new();
    assert_eq!(raw(&fitnesses, 3, &refs, &mut rrng).unwrap(), vec![0, 3, 1]);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

/// Non-finite entries: checked rejects with `InvalidNotFinite`; raw accepts and
/// returns the independently derived rank-1 index. The NaN row and the
/// `[+Inf, +Inf]` row are dominated by the finite row; `[-Inf, -Inf]`
/// dominates it.
#[test]
fn raw_accepts_non_finite_where_checked_rejects() {
    let refs = uniform_reference_points(2, 1).unwrap();

    let nan = vec![vec![1.0, 1.0], vec![f64::NAN, 0.0]];
    let mut crng = CountingPort::new();
    assert_kind(
        checked(&nan, 1, &refs, &mut crng),
        ErrorKind::InvalidNotFinite,
    );
    let mut rrng = CountingPort::new();
    assert_eq!(raw(&nan, 1, &refs, &mut rrng).unwrap(), vec![1]);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);

    let positive = vec![vec![1.0, 1.0], vec![f64::INFINITY, f64::INFINITY]];
    let mut crng = CountingPort::new();
    assert_kind(
        checked(&positive, 1, &refs, &mut crng),
        ErrorKind::InvalidNotFinite,
    );
    let mut rrng = CountingPort::new();
    assert_eq!(raw(&positive, 1, &refs, &mut rrng).unwrap(), vec![0]);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);

    let negative = vec![vec![1.0, 1.0], vec![f64::NEG_INFINITY, f64::NEG_INFINITY]];
    let mut crng = CountingPort::new();
    assert_kind(
        checked(&negative, 1, &refs, &mut crng),
        ErrorKind::InvalidNotFinite,
    );
    let mut rrng = CountingPort::new();
    assert_eq!(raw(&negative, 1, &refs, &mut rrng).unwrap(), vec![1]);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

/// Finite `f64::MAX` inputs whose normalization overflows to a non-finite
/// intermediate: checked rejects at the preserved normalized-matrix finiteness
/// gate; raw skips only that finiteness gate and continues on the same
/// arithmetic, deterministically and without sanitising.
#[test]
fn finite_overflow_intermediate_checked_rejects_raw_continues() {
    let max = f64::MAX;
    let fitnesses = vec![vec![max, -max], vec![-max, max], vec![0.0, 0.0]];
    let refs = uniform_reference_points(2, 1).unwrap();

    let mut crng = CountingPort::new();
    assert_kind(
        checked(&fitnesses, 2, &refs, &mut crng),
        ErrorKind::InvalidNotFinite,
    );
    let mut rrng = CountingPort::new();
    let r = raw(&fitnesses, 2, &refs, &mut rrng).unwrap();
    assert_eq!(r.len(), 2);
    let unique: std::collections::BTreeSet<usize> = r.iter().copied().collect();
    assert_eq!(unique.len(), 2, "no duplicates in the raw survivor set");
    assert!(r.iter().all(|&i| i < 3));
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);

    let mut again = CountingPort::new();
    assert_eq!(raw(&fitnesses, 2, &refs, &mut again).unwrap(), r);
    assert_eq!(again.draws, 0);
}

/// Raw skips finiteness ONLY: structural gates (ragged rows, zero arity, empty
/// reference set, reference arity) keep their exact `ErrorKind` in both routes
/// and precede any draw.
#[test]
fn structural_errors_preserved_in_both_routes() {
    let refs = uniform_reference_points(2, 1).unwrap();
    let empty: Vec<Vec<f64>> = Vec::new();
    let ragged = vec![vec![1.0, 1.0], vec![2.0]];
    let zero_arity = vec![Vec::new(), Vec::new()];
    let partial = vec![vec![1.0, 3.0], vec![2.0, 2.0], vec![3.0, 1.0]];
    let bad_ref = vec![vec![1.0, 0.0, 0.0]];

    let mut crng = CountingPort::new();
    let mut rrng = CountingPort::new();

    assert_kind(
        checked(&ragged, 1, &refs, &mut crng),
        ErrorKind::DimensionMismatch,
    );
    assert_kind(
        raw(&ragged, 1, &refs, &mut rrng),
        ErrorKind::DimensionMismatch,
    );

    assert_kind(
        checked(&zero_arity, 1, &refs, &mut crng),
        ErrorKind::EmptyCollection,
    );
    assert_kind(
        raw(&zero_arity, 1, &refs, &mut rrng),
        ErrorKind::EmptyCollection,
    );

    assert_kind(
        checked(&partial, 2, &empty, &mut crng),
        ErrorKind::EmptyCollection,
    );
    assert_kind(
        raw(&partial, 2, &empty, &mut rrng),
        ErrorKind::EmptyCollection,
    );

    assert_kind(
        checked(&partial, 2, &bad_ref, &mut crng),
        ErrorKind::DimensionMismatch,
    );
    assert_kind(
        raw(&partial, 2, &bad_ref, &mut rrng),
        ErrorKind::DimensionMismatch,
    );

    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

/// Early returns preserve `pop == 0`, `n == 0` and `n >= pop` before any
/// validation, with zero draws and without requiring finite fitness.
#[test]
fn early_returns_precede_validation_in_both_routes() {
    let refs = uniform_reference_points(2, 1).unwrap();
    let frozen = vec![vec![f64::NAN, 0.0]];

    let mut crng = CountingPort::new();
    let mut rrng = CountingPort::new();

    assert!(checked(&[], 3, &refs, &mut crng).unwrap().is_empty());
    assert!(raw(&[], 3, &refs, &mut rrng).unwrap().is_empty());

    assert!(checked(&frozen, 0, &refs, &mut crng).unwrap().is_empty());
    assert!(raw(&frozen, 0, &refs, &mut rrng).unwrap().is_empty());

    // `n >= pop` returns the full index range without validating the NaN row.
    assert_eq!(checked(&frozen, 5, &refs, &mut crng).unwrap(), vec![0]);
    assert_eq!(raw(&frozen, 5, &refs, &mut rrng).unwrap(), vec![0]);

    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

/// Deterministic ordering, length and signed-zero equivalence: repeated raw
/// calls are identical, raw agrees with checked, and `-0.0`/`0.0` rows produce
/// the same derived `[0, 2]` split in both routes.
#[test]
fn deterministic_ordering_length_and_signed_zero() {
    let fitnesses = synthetic_fitnesses(30);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut a = CountingPort::new();
    let mut b = CountingPort::new();
    let first = raw(&fitnesses, 12, &refs, &mut a).unwrap();
    let second = raw(&fitnesses, 12, &refs, &mut b).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 12);

    let mut c = CountingPort::new();
    assert_eq!(checked(&fitnesses, 12, &refs, &mut c).unwrap(), first);

    // Signed zero: the first-seen ideal `[+0, +0]` is retained (strict `<`),
    // translation preserves `-0`, and minimum-count eligibility selects `0`
    // then ref `1` forces `2`, giving `[0, 2]` in both routes.
    let signed = vec![vec![0.0, 1.0], vec![-0.0, 1.0], vec![1.0, 0.0]];
    let refs2 = uniform_reference_points(2, 1).unwrap();
    let mut d = CountingPort::new();
    let mut e = CountingPort::new();
    assert_eq!(checked(&signed, 2, &refs2, &mut d).unwrap(), vec![0, 2]);
    assert_eq!(raw(&signed, 2, &refs2, &mut e).unwrap(), vec![0, 2]);

    assert_eq!(a.draws + b.draws + c.draws + d.draws + e.draws, 0);
}

/// Raw length follows `min(n, pop)` on a larger matrix, matching checked.
#[test]
fn raw_and_checked_lengths_match() {
    let fitnesses = synthetic_fitnesses(40);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut crng = CountingPort::new();
    let mut rrng = CountingPort::new();
    let c = checked(&fitnesses, 15, &refs, &mut crng).unwrap();
    let r = raw(&fitnesses, 15, &refs, &mut rrng).unwrap();
    assert_eq!(c.len(), 15);
    assert_eq!(r.len(), 15);
    assert_eq!(c, r);
    assert_eq!(crng.draws, 0);
    assert_eq!(rrng.draws, 0);
}

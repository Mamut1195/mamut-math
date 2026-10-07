//! Internal helpers shared by the NSGA-III kernels.
//!
//! This module is private (not part of the public API): it centralizes the
//! dynamic-`M` validation, the `f64` <-> core-type conversions and the dispatch
//! to the core port's index sampling ([`RngPort::next_index`]). None of these
//! symbols are exported.

use core::num::NonZeroUsize;

use mamut_math_core::{
    Candidate, ErrorKind, MathError, ObjectiveValues, RngPort, ValidFloat, Variable, VariableKind,
};

/// Validate a dynamic-`M` fitness matrix: `M >= 1`, uniform arity and finite.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `m == 0`.
/// - [`ErrorKind::DimensionMismatch`] when a row arity differs from `m`.
/// - [`ErrorKind::InvalidNotFinite`] when any entry is `NaN`/`±Inf`.
pub(crate) fn validate_fitness_matrix(fitnesses: &[Vec<f64>], m: usize) -> Result<(), MathError> {
    if m == 0 {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "objective vectors must not be empty (M >= 1)",
        ));
    }
    for (row, f) in fitnesses.iter().enumerate() {
        if f.len() != m {
            return Err(MathError::new(
                ErrorKind::DimensionMismatch,
                format!(
                    "ragged fitness matrix: row {row} has arity {} but expected {m}",
                    f.len()
                ),
            ));
        }
        for (column, &value) in f.iter().enumerate() {
            if !value.is_finite() {
                return Err(MathError::new(
                    ErrorKind::InvalidNotFinite,
                    format!("non-finite objective value at row {row}, column {column}"),
                ));
            }
        }
    }
    Ok(())
}

/// Arity of the first row (`0` when the matrix is empty).
pub(crate) fn arity(fitnesses: &[Vec<f64>]) -> usize {
    fitnesses.first().map_or(0, Vec::len)
}

/// Read a core [`Candidate`] back as raw `f64` genes.
pub(crate) fn genes_of(candidate: &Candidate) -> Vec<f64> {
    candidate.as_slice().iter().map(ValidFloat::get).collect()
}

/// Build a core [`Candidate`] from raw `f64` genes, rejecting non-finite values.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] for an empty gene vector.
/// - [`ErrorKind::InvalidNotFinite`] for a non-finite gene.
pub(crate) fn candidate_from_f64(genes: Vec<f64>) -> Result<Candidate, MathError> {
    let values = genes
        .into_iter()
        .map(ValidFloat::new)
        .collect::<Result<Vec<_>, _>>()?;
    Candidate::new(values)
}

/// Read a core [`ObjectiveValues`] back as raw `f64` objectives.
pub(crate) fn objective_vector(objectives: &ObjectiveValues) -> Vec<f64> {
    objectives.iter().map(ValidFloat::get).collect()
}

/// `gen_range(0..n)`-equivalent uniform index sampler over the MAMUT RNG port.
///
/// For `n >= 1` this is a thin dispatcher to [`RngPort::next_index`]: the
/// sampling algorithm (multiply-high with rejection) lives once in the core
/// port, so the operators hold no private sampling body and a consumer can
/// substitute the distribution through an overriding port.
///
/// # Zero bound (historical behavior preserved)
/// - Debug builds: `debug_assert!(n >= 1)` panics before any raw draw.
/// - Release builds: `n == 0` consumes exactly one raw `u64` and returns `0`
///   without evaluating `2^64 % 0`, matching the pre-port private body.
///
/// The pre-existing `u64 -> usize` cast in `operators::sample_gene`
/// (`crates/mamut-nsga3/src/operators.rs`) is unchanged: on a 32-bit target a
/// categorical cardinality of `2^32` still truncates to `0` and reaches this
/// zero path (the historical 32-bit behavior, not a new API contract).
pub(crate) fn gen_index(rng: &mut dyn RngPort, n: usize) -> usize {
    debug_assert!(n >= 1, "gen_index requires n >= 1");
    match NonZeroUsize::new(n) {
        Some(upper) => rng.next_index(upper),
        None => {
            // Historical debug-disabled body: the single raw draw makes the
            // product and its low word `0`, `low < n` is false, so the result
            // is `0` with no `% 0` evaluation.
            let _ = rng.next_u64();
            0
        }
    }
}

/// Raw `f64` interval `[lower, upper]` of a core variable.
///
/// - `Continuous`/`Integer`: their closed bounds (integer bounds cast to `f64`).
/// - `Discrete`: the ordinal interval `[first value, last value]` of the set.
/// - `Categorical`: the index interval `[0, cardinality - 1]`.
///
/// The frozen source only knew float/integer genes; the discrete/categorical
/// treatment is the documented extension rule (extraction contract §B1 row 7 /
/// risk R7) and is pinned by fixtures. Callers decode raw values back to the
/// variable domain as needed.
pub(crate) fn variable_bounds(variable: &Variable) -> (f64, f64) {
    match variable.kind() {
        VariableKind::Continuous { lower, upper } => (lower.get(), upper.get()),
        VariableKind::Integer { lower, upper } => (*lower as f64, *upper as f64),
        VariableKind::Discrete { values } => {
            let vals = values.values();
            (vals[0].get(), vals[vals.len() - 1].get())
        }
        VariableKind::Categorical { cardinality } => (0.0, *cardinality as f64 - 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_math_core::{RngMetadata, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    /// Minimal port using only the pre-port trait methods: it counts raw `u64`
    /// draws and never overrides `next_index`.
    struct CountingPort {
        word_draws: usize,
    }

    impl RngPort for CountingPort {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.word_draws += 1;
            0
        }

        fn next_unit_f64(&mut self) -> f64 {
            0.0
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
            Box::new(CountingPort { word_draws: 0 })
        }
    }

    /// Historical zero-bound behavior, per profile:
    /// - debug: `debug_assert!(n >= 1)` panics before any raw draw;
    /// - release: exactly one raw `u64` is consumed and `0` is returned, with
    ///   no `2^64 % 0` evaluation.
    #[test]
    fn gen_index_zero_preserves_historical_behavior() {
        let mut rng = CountingPort { word_draws: 0 };
        if cfg!(debug_assertions) {
            let outcome = catch_unwind(AssertUnwindSafe(|| gen_index(&mut rng, 0)));
            assert!(outcome.is_err(), "debug: zero bound must panic");
            assert_eq!(
                rng.word_draws, 0,
                "debug: the zero-bound panic must precede any raw draw"
            );
        } else {
            assert_eq!(gen_index(&mut rng, 0), 0, "release: zero bound returns 0");
            assert_eq!(
                rng.word_draws, 1,
                "release: zero bound consumes exactly one raw u64"
            );
        }
    }
}

//! Internal helpers shared by the NSGA-III kernels.
//!
//! This module is private (not part of the public API): it centralizes the
//! dynamic-`M` validation, the `f64` <-> core-type conversions and the
//! `gen_range`-equivalent index sampling. None of these symbols are exported.

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

/// `gen_range(0..n)`-equivalent unbiased index sampler over the MAMUT RNG port.
///
/// Implements the "multiply-high with rejection" (Lemire) method so the result
/// is uniformly distributed over `0..n` without modulo bias. Normally consumes
/// exactly one `u64`; the rejection loop only triggers with probability
/// `< n / 2^64`. `n` must be `>= 1` (documented precondition).
pub(crate) fn gen_index(rng: &mut dyn RngPort, n: usize) -> usize {
    debug_assert!(n >= 1, "gen_index requires n >= 1");
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

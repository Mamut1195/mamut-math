//! Das-Dennis uniform reference points on the unit simplex.
//!
//! Generates the simplex lattice for NSGA-III (Deb & Jain 2014 §3). For `nobj`
//! objectives and `p` divisions it produces `C(nobj + p - 1, p)` reference
//! points, each a normalised direction vector summing to `1.0`.
//!
//! Extracted verbatim from the frozen `nsga3/reference_points.rs` (already
//! dynamic over `nobj`); the only change is call-time validation instead of a
//! `debug_assert!`.

use mamut_math_core::{ErrorKind, MathError};

/// Generate uniform reference points on the unit simplex (Das-Dennis lattice).
///
/// Each returned vector has `nobj` components, all `>= 0.0`, summing to `1.0`.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `nobj == 0` or `p == 0`.
pub fn uniform_reference_points(nobj: usize, p: usize) -> Result<Vec<Vec<f64>>, MathError> {
    if nobj == 0 {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "uniform_reference_points: number of objectives must be >= 1",
        ));
    }
    if p == 0 {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "uniform_reference_points: divisions p must be >= 1",
        ));
    }
    let mut result: Vec<Vec<f64>> = Vec::new();
    let mut current: Vec<usize> = vec![0; nobj];
    gen_refs(nobj, p, 0, p, &mut current, &mut result);
    Ok(result)
}

/// Recursive enumeration of compositions of `remaining` into `nobj - depth`
/// non-negative parts.
fn gen_refs(
    nobj: usize,
    p: usize,
    depth: usize,
    remaining: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<f64>>,
) {
    if depth == nobj - 1 {
        // Last dimension: forced to `remaining`.
        current[depth] = remaining;
        let point: Vec<f64> = current.iter().map(|&v| v as f64 / p as f64).collect();
        result.push(point);
        return;
    }
    for i in 0..=remaining {
        current[depth] = i;
        gen_refs(nobj, p, depth + 1, remaining - i, current, result);
    }
}

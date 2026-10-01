//! Generic Pareto-front quality metrics (all objectives **minimized**).
//!
//! Every kernel here is an algorithm over plain `f64` objective vectors
//! (`&[Vec<f64>]` in, `f64`/`Vec<…>` out) returning the MAMUT-owned core
//! [`mamut_math_core::MathError`] on failure. No third-party and no
//! domain-specific type crosses this boundary.
//!
//! # Orientation
//! **All objectives are minimized.** Maximization callers negate their
//! objective columns before calling; the objective *sense* lives at the core
//! `ObjectiveDescriptor`/session boundary, never on these kernels.
//!
//! # Hypervolume (HV)
//! For a minimization front `A` and reference point `z`, the *dominated
//! hypervolume* is the Lebesgue measure of the region dominated by `A` and
//! bounded above by `z`:
//!
//! ```text
//! HV(A, z) = Λ( ∪_{a ∈ A} [a_1, z_1] × … × [a_M, z_M] )
//! ```
//!
//! [`hypervolume`] evaluates this exactly for arbitrary `M` by the recursive
//! slicing union algorithm (the exact slicing lineage of While, Hingston,
//! Barone & Huband, “A faster algorithm for calculating hypervolume”, *IEEE
//! Transactions on Evolutionary Computation*, 2006). On the last
//! axis the distinct slice boundaries of `A` are collected, the reference level
//! is appended, and each slab `[lower, upper)` accumulates the `(M−1)`-
//! dimensional volume of the active subset scaled by `upper − lower`; the base
//! case `M = 1` is `max(z_1 − min a_1, 0)`. Points not strictly inside the
//! reference box (`a_j ≥ z_j` for some `j`) contribute zero volume and are
//! filtered by definition; duplicate and dominated points leave the union
//! measure unchanged. Cost is `O(N^M)`-class in front size `N` and dimension
//! `M`; for large `N`/`M`, callers subsample or evaluate offline — the library
//! never approximates silently.
//!
//! # IGD+ (Inverted Generational Distance Plus)
//! [`igd_plus`] implements the Pareto-respecting IGD+ of Ishibuchi, Masuda,
//! Tanigaki & Nojima (2015):
//!
//! ```text
//! IGD+(A, R) = (1/|R|) · Σ_{r ∈ R} min_{a ∈ A} d+(a, r)
//! d+(a, r)   = sqrt( Σ_j max(a_j − r_j, 0)^2 )
//! ```
//!
//! where only the objectives on which `a` is *worse* than `r` contribute
//! (`max(a_j − r_j, 0)`; the minimization sign is frozen from the parity-
//! corrected source). **Caller obligation:** `R` must be a reference subset of
//! the true Pareto-optimal front; IGD+ is only meaningful against such a
//! reference. Degenerate cases are pinned: `igd_plus(F, F) == 0` exactly, and
//! an empty approximation or reference front is an
//! [`mamut_math_core::ErrorKind::EmptyCollection`] (the frozen `∞`
//! placeholder is deliberately replaced).
//!
//! # Normalization helpers
//! [`normalize_to_unit_box`] / [`denormalize_from_unit_box`] apply (and
//! invert) a per-objective min–max affine map into `[0, 1]^M` using explicit
//! bounds `lower`/`upper` (callers typically derive these as the per-objective
//! min/max of the front). They are **not** the NSGA-III hyperplane-intercept
//! normalizer ([`crate::normalize()`]): they never consult a reference lattice
//! and introduce no hidden scaling. A zero-width (`upper == lower`) or inverted
//! (`upper < lower`) range is a typed error, never a silent division.
//!
//! # Validation matrix
//! Every entry point validates via the private `crate::internal` helpers
//! (`M ≥ 1`, uniform arity, finite) plus its own reference/bounds arity check.
//! Ragged or
//! non-finite input is never filtered — it is
//! [`mamut_math_core::ErrorKind::DimensionMismatch`] /
//! [`mamut_math_core::ErrorKind::InvalidNotFinite`]. See the placement
//! decision `docs/extraction/NSGA003-PLACEMENT.md` §3 for the full case matrix.
//!
//! # Oracle provenance
//! The 5-objective golden fixture (`tests/fixtures/metrics_5d_golden.json`,
//! `HV = 784.0`, `IGD+ = 0.6889635872182424`, absolute tolerance `1e-12`) was
//! generated offline with **DEAP exact HV** and an **independent Python IGD+**,
//! and is re-encoded from the frozen Hydraulics fixture at
//! `6db4862d7a4035de8d494cdc62607b12b4edb103` (read-only).

use mamut_math_core::{ErrorKind, MathError};

use crate::internal;

/// Exact dominated hypervolume of `front` with respect to `reference_point`.
///
/// All objectives are minimized. The result is the measure of the union of the
/// point-to-reference boxes; an empty `front` has measure `0.0` (a
/// mathematically defined value, not a placeholder). Points that are not
/// strictly inside the reference box, duplicates and dominated points leave the
/// measure unchanged (see the module docs).
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `reference_point` is empty.
/// - [`ErrorKind::InvalidNotFinite`] when any reference or front entry is
///   `NaN`/`±Inf`.
/// - [`ErrorKind::DimensionMismatch`] when a front row's arity differs from
///   `reference_point.len()`.
pub fn hypervolume(front: &[Vec<f64>], reference_point: &[f64]) -> Result<f64, MathError> {
    if reference_point.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "hypervolume: reference point must have M >= 1 objectives",
        ));
    }
    for (axis, &value) in reference_point.iter().enumerate() {
        if !value.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("hypervolume: non-finite reference component at axis {axis}"),
            ));
        }
    }

    let m = reference_point.len();
    internal::validate_fitness_matrix(front, m)?;

    if front.is_empty() {
        return Ok(0.0);
    }

    // Points outside the reference box (a_j >= z_j on some axis) span a box of
    // non-positive extent and contribute zero volume; drop them before slicing.
    let points: Vec<Vec<f64>> = front
        .iter()
        .filter(|point| {
            point
                .iter()
                .zip(reference_point)
                .all(|(value, reference)| value < reference)
        })
        .cloned()
        .collect();

    Ok(hypervolume_slices(&points, reference_point))
}

/// Recursive exact-slicing union volume (frozen arithmetic, generalized-`M`).
fn hypervolume_slices(points: &[Vec<f64>], reference_point: &[f64]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }

    if reference_point.len() == 1 {
        let minimum = points
            .iter()
            .map(|point| point[0])
            .fold(reference_point[0], f64::min);
        return (reference_point[0] - minimum).max(0.0);
    }

    let axis = reference_point.len() - 1;
    let mut levels: Vec<f64> = points.iter().map(|point| point[axis]).collect();
    levels.sort_by(|a, b| a.total_cmp(b));
    levels.dedup_by(|a, b| a.total_cmp(b).is_eq());
    levels.push(reference_point[axis]);

    let mut volume = 0.0;
    for bounds in levels.windows(2) {
        let lower = bounds[0];
        let upper = bounds[1];
        if upper <= lower {
            continue;
        }

        let active: Vec<Vec<f64>> = points
            .iter()
            .filter(|point| point[axis] <= lower)
            .map(|point| point[..axis].to_vec())
            .collect();
        let slice = hypervolume_slices(&active, &reference_point[..axis]);
        volume += slice * (upper - lower);
    }

    volume
}

/// IGD+ (Ishibuchi 2015) of `front` against an explicit `reference_front`.
///
/// All objectives are minimized. `d+(a, r) = sqrt(Σ_j max(a_j − r_j, 0)^2)` and
/// `IGD+(A, R) = (1/|R|)·Σ_{r∈R} min_{a∈A} d+(a, r)`. `igd_plus(F, F) == 0`
/// exactly. **Caller obligation:** `reference_front` must be (a subset of) the
/// true Pareto-optimal front for the value to be a meaningful quality measure.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `front` or `reference_front` is empty.
/// - [`ErrorKind::InvalidNotFinite`] on `NaN`/`±Inf` entries.
/// - [`ErrorKind::DimensionMismatch`] when a `reference_front` row does not
///   match the arity `M` of `front`.
pub fn igd_plus(front: &[Vec<f64>], reference_front: &[Vec<f64>]) -> Result<f64, MathError> {
    if front.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "igd_plus: approximation front must be non-empty",
        ));
    }
    if reference_front.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "igd_plus: reference front must be non-empty",
        ));
    }

    let m = internal::arity(front);
    internal::validate_fitness_matrix(front, m)?;
    internal::validate_fitness_matrix(reference_front, m)?;

    let mut sum = 0.0_f64;
    for r in reference_front {
        let min_dist = front
            .iter()
            .map(|a| {
                // d+(a, r): only axes where the approximation is worse (higher)
                // than the reference contribute; better axes are clipped to 0.
                let sq_sum: f64 = r
                    .iter()
                    .zip(a.iter())
                    .map(|(&rj, &aj)| {
                        let diff = aj - rj;
                        if diff > 0.0 {
                            diff * diff
                        } else {
                            0.0
                        }
                    })
                    .sum();
                sq_sum.sqrt()
            })
            .fold(f64::INFINITY, f64::min);
        sum += min_dist;
    }

    Ok(sum / reference_front.len() as f64)
}

/// Min–max affine map of `front` into `[0, 1]^M` using explicit bounds.
///
/// `out[i][j] = (front[i][j] − lower[j]) / (upper[j] − lower[j])`. All
/// objectives are minimized; the map is monotone per objective. This is a
/// metric-local rescaling, **not** the NSGA-III intercept normalizer
/// ([`crate::normalize()`]); callers typically pass the front's own per-objective
/// min/max. The map does not clip: points outside `[lower, upper]` map outside
/// `[0, 1]` (documented affine behavior, no silent clamp).
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `front` is empty.
/// - [`ErrorKind::DimensionMismatch`] when `lower`/`upper` arity differs from
///   the front arity, or the front is ragged.
/// - [`ErrorKind::InvalidNotFinite`] on non-finite front or bound entries.
/// - [`ErrorKind::InvertedBounds`] when `upper[j] < lower[j]`.
/// - [`ErrorKind::DomainMismatch`] when `upper[j] == lower[j]` (a constant
///   objective has no unit-box image — a zero-width range would divide by
///   zero, so it is rejected rather than silently producing `NaN`/`Inf`).
pub fn normalize_to_unit_box(
    front: &[Vec<f64>],
    lower: &[f64],
    upper: &[f64],
) -> Result<Vec<Vec<f64>>, MathError> {
    let m = validate_bounds(front, lower, upper, "normalize_to_unit_box")?;
    let mut out = Vec::with_capacity(front.len());
    for row in front {
        let mapped: Vec<f64> = row
            .iter()
            .zip(lower.iter())
            .zip(upper.iter())
            .map(|((&value, &lo), &hi)| (value - lo) / (hi - lo))
            .collect();
        debug_assert_eq!(mapped.len(), m);
        out.push(mapped);
    }
    Ok(out)
}

/// Inverse of [`normalize_to_unit_box`].
///
/// `out[i][j] = normed[i][j] · (upper[j] − lower[j]) + lower[j]`. The exact
/// affine inverse: `denormalize(normalize(f, lo, hi), lo, hi) == f` up to
/// floating-point round-off. Values of `normed` outside `[0, 1]` are accepted
/// and mapped affinely (no clamp).
///
/// # Errors
/// Identical to [`normalize_to_unit_box`] (`EmptyCollection`,
/// `DimensionMismatch`, `InvalidNotFinite`, `InvertedBounds`,
/// `DomainMismatch`).
pub fn denormalize_from_unit_box(
    normed: &[Vec<f64>],
    lower: &[f64],
    upper: &[f64],
) -> Result<Vec<Vec<f64>>, MathError> {
    let m = validate_bounds(normed, lower, upper, "denormalize_from_unit_box")?;
    let mut out = Vec::with_capacity(normed.len());
    for row in normed {
        let mapped: Vec<f64> = row
            .iter()
            .zip(lower.iter())
            .zip(upper.iter())
            .map(|((&value, &lo), &hi)| value * (hi - lo) + lo)
            .collect();
        debug_assert_eq!(mapped.len(), m);
        out.push(mapped);
    }
    Ok(out)
}

/// Shared validation for the two bound-based maps: non-empty front, finite and
/// well-ordered bounds of matching arity. Returns the front arity `M`.
fn validate_bounds(
    front: &[Vec<f64>],
    lower: &[f64],
    upper: &[f64],
    context: &str,
) -> Result<usize, MathError> {
    if front.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            format!("{context}: front must be non-empty"),
        ));
    }

    let m = internal::arity(front);
    internal::validate_fitness_matrix(front, m)?;

    if lower.len() != m || upper.len() != m {
        return Err(MathError::new(
            ErrorKind::DimensionMismatch,
            format!(
                "{context}: bounds arity (lower {}, upper {}) != front arity {m}",
                lower.len(),
                upper.len()
            ),
        ));
    }

    for (axis, (&lo, &hi)) in lower.iter().zip(upper.iter()).enumerate() {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("{context}: non-finite bound at axis {axis}"),
            ));
        }
        if hi < lo {
            return Err(MathError::new(
                ErrorKind::InvertedBounds,
                format!("{context}: upper {hi} < lower {lo} at axis {axis}"),
            ));
        }
        if hi == lo {
            return Err(MathError::new(
                ErrorKind::DomainMismatch,
                format!(
                    "{context}: zero-width range at axis {axis} \
                     (constant objective has no unit-box image)"
                ),
            ));
        }
    }

    Ok(m)
}

//! NSGA-III normalization — extreme-point hyperplane intercept method.
//!
//! Implements Deb & Jain 2014 §4.1:
//! 1. Translate all objectives by the **ideal point** (per-objective minimum).
//! 2. Find **extreme points** via the Achievement Scalarizing Function (ASF).
//! 3. Solve an `M×M` linear system for hyperplane **intercepts**.
//! 4. If the system is degenerate (`|pivot| < 1e-12`) or any intercept `<= 0`,
//!    fall back to the per-objective maximum of the translated population.
//! 5. Divide translated objectives by intercepts → normalized objectives.
//!
//! Extracted from the frozen `nsga3/normalize.rs`, generalizing the hand-coded
//! `5×5` Gaussian elimination to `M×M`. Algorithmic constants (`1e-12`, `1e-6`),
//! accumulation order and the pivot guard are preserved verbatim.

use mamut_math_core::MathError;

use crate::ieee_selection::FinitePolicy;
use crate::internal;

/// Result of the normalization step.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizeResult {
    /// Translated and intercept-divided fitness, one row per individual.
    pub normalized: Vec<Vec<f64>>,
    /// The ideal point (per-objective minimum) used for translation.
    pub ideal: Vec<f64>,
    /// The hyperplane intercepts (after fallback if needed).
    pub intercepts: Vec<f64>,
}

/// Normalize a set of `M`-objective fitness vectors (all minimized).
///
/// Empty input returns an empty result: with no rows the arity `M` is not
/// inferable, so `normalized`/`ideal`/`intercepts` are all zero-length (the
/// frozen fixed-`[f64; 5]` zero/one defaults have no dynamic-`M` analogue).
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when rows have `M == 0`.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] on ragged input.
/// - [`mamut_math_core::ErrorKind::InvalidNotFinite`] on non-finite entries.
pub fn normalize(fitnesses: &[Vec<f64>]) -> Result<NormalizeResult, MathError> {
    normalize_kernel(fitnesses, FinitePolicy::Checked)
}

/// Shared checked/raw normalization body.
///
/// The [`FinitePolicy`] toggles only the finiteness gates (the input matrix and
/// the intercept solve). The ideal point, the ASF extreme points, the Gaussian
/// elimination and the intercept-divide arithmetic are identical for both
/// routes.
pub(crate) fn normalize_kernel(
    fitnesses: &[Vec<f64>],
    policy: FinitePolicy,
) -> Result<NormalizeResult, MathError> {
    let n = fitnesses.len();
    if n == 0 {
        return Ok(NormalizeResult {
            normalized: Vec::new(),
            ideal: Vec::new(),
            intercepts: Vec::new(),
        });
    }

    let m = internal::arity(fitnesses);
    policy.validate_matrix(fitnesses, m)?;

    // ── Step 1: ideal point (per-objective minimum) ─────────────────────
    let mut ideal = vec![f64::INFINITY; m];
    for f in fitnesses {
        for j in 0..m {
            if f[j] < ideal[j] {
                ideal[j] = f[j];
            }
        }
    }

    // ── Step 2: translate by ideal ──────────────────────────────────────
    let translated: Vec<Vec<f64>> = fitnesses
        .iter()
        .map(|f| {
            let mut t = vec![0.0_f64; m];
            for j in 0..m {
                t[j] = f[j] - ideal[j];
            }
            t
        })
        .collect();

    // ── Step 3: extreme points via ASF ──────────────────────────────────
    let extreme_points = find_extreme_points(&translated, m);

    // ── Step 4: solve M×M system for intercepts ─────────────────────────
    let intercepts = compute_intercepts(&extreme_points, &translated, m, policy);

    // ── Step 5: divide by intercepts ────────────────────────────────────
    let normalized: Vec<Vec<f64>> = translated
        .iter()
        .map(|t| {
            let mut normed = vec![0.0_f64; m];
            for j in 0..m {
                normed[j] = if intercepts[j] > 1e-12 {
                    t[j] / intercepts[j]
                } else {
                    t[j]
                };
            }
            normed
        })
        .collect();

    Ok(NormalizeResult {
        normalized,
        ideal,
        intercepts,
    })
}

/// Find the extreme point for each objective axis via the ASF.
fn find_extreme_points(translated: &[Vec<f64>], m: usize) -> Vec<Vec<f64>> {
    let mut extremes = vec![vec![0.0_f64; m]; m];
    for (axis, extreme) in extremes.iter_mut().enumerate() {
        let mut best_asf = f64::INFINITY;
        let mut best_point = vec![0.0_f64; m];
        for t in translated {
            let asf = compute_asf(t, axis);
            if asf < best_asf {
                best_asf = asf;
                best_point = t.clone();
            }
        }
        *extreme = best_point;
    }
    extremes
}

/// Achievement Scalarizing Function for a translated fitness vector.
///
/// `w_j = 1` for `j == axis`, `1e-6` otherwise; `ASF(f) = max_j(f_j / w_j)`.
#[inline]
fn compute_asf(f: &[f64], axis: usize) -> f64 {
    f.iter()
        .enumerate()
        .map(|(j, &fj)| {
            let w = if j == axis { 1.0_f64 } else { 1e-6_f64 };
            fj / w
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Compute intercepts by solving the `M×M` linear system `A · x = 1`.
///
/// Falls back to per-objective maximum when the system is degenerate
/// (`|pivot| < 1e-12`) or any intercept is `<= 0`.
fn compute_intercepts(
    extreme_points: &[Vec<f64>],
    translated: &[Vec<f64>],
    m: usize,
    policy: FinitePolicy,
) -> Vec<f64> {
    // Build augmented matrix [A | b] where b = [1; 1; ...; 1].
    let mut a = vec![vec![0.0_f64; m + 1]; m];
    for i in 0..m {
        for j in 0..m {
            a[i][j] = extreme_points[i][j];
        }
        a[i][m] = 1.0;
    }

    // Gaussian elimination with partial pivoting. Index-based algorithm
    // (row/column indices used simultaneously for pivoting and back-substitution).
    #[allow(clippy::needless_range_loop)]
    for col in 0..m {
        // Find pivot row (partial pivoting).
        let mut max_row = col;
        let mut max_val = a[col][col].abs();
        for row in (col + 1)..m {
            if a[row][col].abs() > max_val {
                max_val = a[row][col].abs();
                max_row = row;
            }
        }
        a.swap(col, max_row);

        if a[col][col].abs() < 1e-12 {
            // Degenerate system — fall back to max-per-objective.
            return fallback_intercepts(translated, m);
        }

        // Eliminate below pivot row.
        for row in (col + 1)..m {
            let factor = a[row][col] / a[col][col];
            // Copy pivot row to avoid simultaneous borrow of `a`.
            let pivot: Vec<f64> = a[col].clone();
            for (elem, &piv) in a[row][col..].iter_mut().zip(pivot[col..].iter()) {
                *elem -= factor * piv;
            }
        }
    }

    // Back-substitution.
    let mut x = vec![0.0_f64; m];
    for i in (0..m).rev() {
        x[i] = a[i][m];
        for j in (i + 1)..m {
            x[i] -= a[i][j] * x[j];
        }
        x[i] /= a[i][i];
    }

    // Intercepts: a_j = 1 / x_j.
    let mut intercepts = vec![0.0_f64; m];
    for j in 0..m {
        // Checked route rejects a non-finite solve component (finiteness gate);
        // the raw route skips ONLY this check and lets the IEEE value flow.
        if x[j] <= 0.0 || (policy.checks_finite() && !x[j].is_finite()) {
            return fallback_intercepts(translated, m);
        }
        intercepts[j] = 1.0 / x[j];
    }
    intercepts
}

/// Fallback: per-objective maximum of the translated population.
fn fallback_intercepts(translated: &[Vec<f64>], m: usize) -> Vec<f64> {
    let mut max_vals = vec![f64::NEG_INFINITY; m];
    for t in translated {
        for (max, &v) in max_vals.iter_mut().zip(t.iter()) {
            if v > *max {
                *max = v;
            }
        }
    }
    // Guard against zero intercepts (all-identical objectives on an axis).
    for max in &mut max_vals {
        if *max <= 0.0 {
            *max = 1.0;
        }
    }
    max_vals
}

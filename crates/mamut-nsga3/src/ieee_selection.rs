//! Explicit IEEE-754 `f64` (raw) environmental-selection route.
//!
//! This is the **compatibility low-level** entry point for NSGA-III
//! environmental selection. It exposes
//! [`select_environmental`] over raw `f64` fitness vectors, preserving
//! unsanitised `NaN`/`±Inf` entries instead of rejecting them through the
//! finite validation gates of the checked [`crate::selection`] route.
//!
//! ## Contract boundary (not an engineering-validity promise)
//! - [`crate::selection::select_environmental`] keeps the checked path: every
//!   fitness matrix is validated for finiteness at the original gates and a
//!   non-finite entry is reported as
//!   [`mamut_math_core::ErrorKind::InvalidNotFinite`].
//! - This module is a *compatibility* route: it shares the **same** arithmetic
//!   body — nondominated sort order, normalization (ideal/intercepts/divide),
//!   association/perp-distance and niche selection — as the checked path, with
//!   the same structural checks, early returns and zero RNG consumption, but it
//!   does **not** finite-validate the fitness matrix, the reference points or
//!   the normalized intermediates.
//! - `NaN` ordering, `NaN`-in/`NaN`-out propagation and any downstream
//!   engineering or objective validity are **not** promised here. A consumer
//!   that needs finite fitness must validate it itself or keep using the typed
//!   path. Structural shape (empty collection, ragged rows, reference arity) and
//!   typed [`mamut_math_core::MathError`] results are preserved exactly.

use mamut_math_core::{ErrorKind, MathError, RngPort};

use crate::internal;
use crate::selection;

/// Crate-private finiteness policy shared by the checked and raw selection
/// kernels.
///
/// The policy is the *only* difference between the two routes: it decides
/// whether the finiteness gates run. The arithmetic (sort, normalization,
/// association, niche selection), the structural gates, the early returns and
/// the RNG timing are otherwise one shared body per kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FinitePolicy {
    /// Checked public route: non-finite values are rejected at the original
    /// finiteness gates.
    Checked,
    /// IEEE raw route: finiteness is not enforced; the structural gates are
    /// kept and the arithmetic is otherwise identical.
    Raw,
}

impl FinitePolicy {
    /// Whether this policy enforces the finiteness gates.
    #[inline]
    pub(crate) const fn checks_finite(self) -> bool {
        matches!(self, FinitePolicy::Checked)
    }

    /// Validate a dynamic-`M` fitness matrix under this policy.
    ///
    /// [`FinitePolicy::Checked`] delegates verbatim to
    /// [`crate::internal::validate_fitness_matrix`], preserving the exact
    /// finiteness error; [`FinitePolicy::Raw`] performs only the structural
    /// checks (`M >= 1`, uniform arity), skipping the finiteness gate.
    #[inline]
    pub(crate) fn validate_matrix(self, fitnesses: &[Vec<f64>], m: usize) -> Result<(), MathError> {
        match self {
            FinitePolicy::Checked => internal::validate_fitness_matrix(fitnesses, m),
            FinitePolicy::Raw => validate_matrix_structure(fitnesses, m),
        }
    }
}

/// Structural-only matrix validation for the raw route: `M >= 1` and uniform
/// arity, with the finiteness gate deliberately omitted.
///
/// The structural error kinds and messages mirror
/// [`crate::internal::validate_fitness_matrix`] so the raw route preserves the
/// same structural failures and their order. This is the raw counterpart of the
/// checked finiteness gate, not a second arithmetic body.
pub(crate) fn validate_matrix_structure(fitnesses: &[Vec<f64>], m: usize) -> Result<(), MathError> {
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
    }
    Ok(())
}

/// Select `n` individuals from a population using the raw IEEE environmental
/// selection route.
///
/// This is the single raw environmental-selection seam: it drives the same
/// `nondom_sort -> normalize -> associate -> niche-select` chain as
/// [`crate::selection::select_environmental`] through the shared finite-policy
/// kernels.
///
/// # Arguments
/// * `fitnesses` — raw fitness matrix, one `M`-objective row per individual.
/// * `n` — target selection size.
/// * `ref_points` — reference point vectors from the Das-Dennis lattice.
/// * `rng` — reserved (as in the checked route) for future stochastic
///   tie-breaking; currently **unconsumed**. This route performs no draws.
///
/// # Returns
/// A vector of length `min(n, fitnesses.len())` of selected indices.
///
/// # Early returns (before any validation)
/// - `fitnesses.is_empty()` or `n == 0` returns an empty vector.
/// - `n >= fitnesses.len()` returns `0..fitnesses.len()`.
///
/// # Errors
/// Structural errors are preserved exactly and are the only errors this route
/// raises; the finiteness gates of the checked route are skipped.
/// - [`ErrorKind::EmptyCollection`] when `M == 0` or `ref_points` is empty.
/// - [`ErrorKind::DimensionMismatch`] on ragged fitness rows or reference
///   arity mismatch.
///
/// # IEEE behaviour (no sanitisation promise)
/// Non-finite fitness, reference or intermediate values flow through the shared
/// arithmetic untouched. `NaN` ordering and propagation follow the underlying
/// IEEE-754 operations; no `NaN` payload, sentinel substitution or finiteness
/// guarantee is provided.
pub fn select_environmental(
    fitnesses: &[Vec<f64>],
    n: usize,
    ref_points: &[Vec<f64>],
    rng: &mut dyn RngPort,
) -> Result<Vec<usize>, MathError> {
    selection::select_environmental_kernel(fitnesses, n, ref_points, rng, FinitePolicy::Raw)
}

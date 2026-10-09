//! NSGA-III environmental selection — top-level wrapper.
//!
//! Combines:
//! 1. Fast nondominated sort.
//! 2. Front-by-front fill until the "splitting" front.
//! 3. Normalize objectives.
//! 4. Associate + niche-count selection from the partial front.
//!
//! Extracted from the frozen `nsga3/selection.rs`, generalized to dynamic-`M`.
//! The frozen `sel_nsga3` convenience wrapper (which fixes 5 objectives and
//! `p = 4`) is **not** ported: that lattice choice is caller policy, and a
//! consumer re-expresses it as
//! `select_environmental(fitnesses, n, &uniform_reference_points(5, 4)?, rng)`.

use mamut_math_core::MathError;
use mamut_math_core::RngPort;

use crate::ieee_selection::FinitePolicy;
use crate::internal;
use crate::niching::{associate_kernel, build_niche_counts, select_from_partial_front};
use crate::nondom_sort::fast_nondominated_sort_kernel;
use crate::normalize::normalize_kernel;

/// Select `n` individuals from a population using NSGA-III environmental
/// selection.
///
/// # Arguments
/// * `fitnesses` — fitness matrix, one `M`-objective row per individual.
/// * `n` — target selection size.
/// * `ref_points` — reference point vectors from the Das-Dennis lattice.
/// * `_rng` — reserved for future stochastic tie-breaking; currently unconsumed
///   (the deterministic smallest-index tie-break is used). It remains a
///   MAMUT-owned [`RngPort`] so a future ticket can consume it without an API
///   break.
///
/// # Returns
/// A vector of length `min(n, fitnesses.len())` of selected indices.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `M == 0` or `refs` is
///   empty (and `n < pop`).
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] on ragged fitness rows or
///   reference arity mismatch.
/// - [`mamut_math_core::ErrorKind::InvalidNotFinite`] on non-finite entries.
pub fn select_environmental(
    fitnesses: &[Vec<f64>],
    n: usize,
    ref_points: &[Vec<f64>],
    _rng: &mut dyn RngPort,
) -> Result<Vec<usize>, MathError> {
    select_environmental_kernel(fitnesses, n, ref_points, _rng, FinitePolicy::Checked)
}

/// Shared checked/raw environmental-selection body.
///
/// This is the single arithmetic body for both routes. The [`FinitePolicy`]
/// only toggles the finiteness gates: the structural checks, the early returns
/// and the sort/normalize/associate/niche arithmetic are identical.
pub(crate) fn select_environmental_kernel(
    fitnesses: &[Vec<f64>],
    n: usize,
    ref_points: &[Vec<f64>],
    _rng: &mut dyn RngPort,
    policy: FinitePolicy,
) -> Result<Vec<usize>, MathError> {
    let pop_size = fitnesses.len();

    if pop_size == 0 || n == 0 {
        return Ok(Vec::new());
    }
    if n >= pop_size {
        return Ok((0..pop_size).collect());
    }

    let m = internal::arity(fitnesses);
    policy.validate_matrix(fitnesses, m)?;

    // ── Step 1: fast nondominated sort ──────────────────────────────────
    let fronts = fast_nondominated_sort_kernel(fitnesses, policy)?;

    // ── Step 2: fill front-by-front until the splitting front ───────────
    let mut selected: Vec<usize> = Vec::with_capacity(n);
    let mut partial_front: Vec<usize> = Vec::new();

    for front in &fronts {
        if selected.len() + front.len() <= n {
            // Entire front fits.
            selected.extend_from_slice(front);
        } else {
            // This is the splitting (partial) front.
            partial_front = front.clone();
            break;
        }
        if selected.len() == n {
            return Ok(selected);
        }
    }

    // Already have exactly n after full fronts (no partial needed).
    if selected.len() == n {
        return Ok(selected);
    }

    let needed = n - selected.len();

    // ── Step 3: normalize ALL candidates (definite + partial) ───────────
    let all_candidates: Vec<usize> = selected
        .iter()
        .copied()
        .chain(partial_front.iter().copied())
        .collect();

    let candidate_fitnesses: Vec<Vec<f64>> = all_candidates
        .iter()
        .map(|&i| fitnesses[i].clone())
        .collect();

    let norm_result = normalize_kernel(&candidate_fitnesses, policy)?;

    // ── Step 4: associate ALL candidates to reference points ────────────
    // Association table indexed by ORIGINAL population index.
    // Unused entries default to (0, f64::INFINITY).
    let mut assoc: Vec<(usize, f64)> = vec![(0, f64::INFINITY); pop_size];
    let candidate_assoc = associate_kernel(&norm_result.normalized, ref_points, policy)?;
    for (pos, &orig_idx) in all_candidates.iter().enumerate() {
        assoc[orig_idx] = candidate_assoc[pos];
    }

    // ── Step 5: niche counts from the definite set ──────────────────────
    let mut niche_count = build_niche_counts(&selected, &assoc, ref_points.len());

    // ── Step 6: select from the partial front via niching ───────────────
    let extra = select_from_partial_front(&partial_front, &assoc, &mut niche_count, needed);

    selected.extend(extra);
    Ok(selected)
}

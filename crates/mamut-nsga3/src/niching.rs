//! NSGA-III niching — association and niche-count selection.
//!
//! Implements Deb & Jain 2014 §4.2:
//! 1. **Associate** each individual to the nearest reference point (minimum
//!    perpendicular distance to the reference line through the origin).
//! 2. Build **niche counts** over the "definite" set.
//! 3. **Select** from the partial (splitting) front preferring the reference
//!    with the lowest niche count; tie-break by minimum perpendicular distance,
//!    then by smallest index (deterministic).
//!
//! Extracted from the frozen `nsga3/niching.rs`, generalized from `&[f64; 5]`
//! to slices. Per extraction-contract decision H3 the reserved, unconsumed
//! `rng` parameter of `select_from_partial_front` is **dropped** (it was never
//! read; a future stochastic tie-break ticket re-adds `&mut dyn RngPort`).

use mamut_math_core::{ErrorKind, MathError};

use crate::internal;

/// Perpendicular distance from point `p` to the reference line through the
/// origin defined by direction vector `r`.
///
/// `d_perp = sqrt(||p||² − (p·r)² / ||r||²)`. The degenerate-reference guard is
/// `||r||² < 1e-30`, which returns `||p||`.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `p` is empty.
/// - [`ErrorKind::DimensionMismatch`] when `p.len() != r.len()`.
/// - [`ErrorKind::InvalidNotFinite`] when any component of `p` or `r` is
///   `NaN`/`±Inf` (the degenerate `1e-30` guard applies only to finite inputs).
pub fn perp_distance(p: &[f64], r: &[f64]) -> Result<f64, MathError> {
    if p.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "perp_distance: point must have at least one component",
        ));
    }
    if p.len() != r.len() {
        return Err(MathError::new(
            ErrorKind::DimensionMismatch,
            format!(
                "perp_distance: point arity {} does not match reference arity {}",
                p.len(),
                r.len()
            ),
        ));
    }
    for (index, &value) in p.iter().enumerate() {
        if !value.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("perp_distance: non-finite point component at [{index}]"),
            ));
        }
    }
    for (index, &value) in r.iter().enumerate() {
        if !value.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("perp_distance: non-finite reference component at [{index}]"),
            ));
        }
    }

    // dot(p, r)
    let dot_pr: f64 = p.iter().zip(r).map(|(a, b)| a * b).sum();
    // ||r||²
    let r_sq: f64 = r.iter().map(|x| x * x).sum();

    if r_sq < 1e-30 {
        // Degenerate reference — distance = ||p||.
        return Ok(p.iter().map(|x| x * x).sum::<f64>().sqrt());
    }

    // ||p||²
    let p_sq: f64 = p.iter().map(|x| x * x).sum();
    // d² = ||p||² − (p·r)²/||r||²
    let d_sq = (p_sq - dot_pr * dot_pr / r_sq).max(0.0);
    Ok(d_sq.sqrt())
}

/// Associate each normalized individual to its nearest reference point.
///
/// Returns a `(ref_idx, distance)` pair per individual.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `refs` is empty.
/// - [`ErrorKind::DimensionMismatch`] when a reference arity differs from the
///   normalized arity `M`.
/// - [`ErrorKind::EmptyCollection`]/[`ErrorKind::InvalidNotFinite`] for
///   `M == 0` or non-finite normalized/reference entries.
pub fn associate(
    normalized: &[Vec<f64>],
    refs: &[Vec<f64>],
) -> Result<Vec<(usize, f64)>, MathError> {
    if refs.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "associate: at least one reference point is required",
        ));
    }
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let m = internal::arity(normalized);
    internal::validate_fitness_matrix(normalized, m)?;
    for (index, r) in refs.iter().enumerate() {
        if r.len() != m {
            return Err(MathError::new(
                ErrorKind::DimensionMismatch,
                format!(
                    "associate: reference {index} has arity {} but normalized arity is {m}",
                    r.len()
                ),
            ));
        }
        for (j, &value) in r.iter().enumerate() {
            if !value.is_finite() {
                return Err(MathError::new(
                    ErrorKind::InvalidNotFinite,
                    format!("associate: non-finite reference entry at {index}, {j}"),
                ));
            }
        }
    }

    let mut assoc = Vec::with_capacity(normalized.len());
    for p in normalized {
        let mut best_ref = 0_usize;
        let mut best_dist = f64::INFINITY;
        for (ref_index, r) in refs.iter().enumerate() {
            let d = perp_distance(p, r)?;
            if d < best_dist {
                best_dist = d;
                best_ref = ref_index;
            }
        }
        assoc.push((best_ref, best_dist));
    }
    Ok(assoc)
}

/// Build niche counts for a subset of individuals (the "definite" set).
///
/// `assoc` is indexed by original individual index and must have length
/// `> max(subset)`; `num_refs` is the number of reference points.
///
/// # Panics
/// Panics if a subset index is not present in `assoc` or if its associated
/// reference index is `>= num_refs` (documented precondition; the selection
/// wrapper always satisfies it).
#[must_use]
pub fn build_niche_counts(subset: &[usize], assoc: &[(usize, f64)], num_refs: usize) -> Vec<u32> {
    let mut counts = vec![0_u32; num_refs];
    for &idx in subset {
        let (ref_idx, _) = assoc[idx];
        counts[ref_idx] += 1;
    }
    counts
}

/// Select `needed` individuals from `partial_front` to fill the next population.
///
/// Niche-count-based selection:
/// - find the reference `j*` with minimum niche count among references
///   associated to the remaining partial-front individuals;
/// - among those associated with `j*`, pick the one with minimum perpendicular
///   distance (tie-break: smallest individual index, deterministic);
/// - increment the niche count for `j*` and repeat.
///
/// # Panics
/// Panics if a partial-front index is not present in `assoc` or its reference
/// index is outside `niche_count` (documented precondition).
#[must_use]
pub fn select_from_partial_front(
    partial_front: &[usize],
    assoc: &[(usize, f64)],
    niche_count: &mut [u32],
    needed: usize,
) -> Vec<usize> {
    let mut remaining: Vec<usize> = partial_front.to_vec();
    let mut selected: Vec<usize> = Vec::with_capacity(needed);

    while selected.len() < needed && !remaining.is_empty() {
        // Find the minimum niche count among references associated to remaining.
        let min_nc = remaining
            .iter()
            .map(|&idx| niche_count[assoc[idx].0])
            .min()
            .unwrap_or(0);

        // Candidates associated with a ref point that has the minimum count.
        let candidates: Vec<usize> = remaining
            .iter()
            .copied()
            .filter(|&idx| niche_count[assoc[idx].0] == min_nc)
            .collect();

        // Among candidates, pick the smallest perpendicular distance; tie-break
        // by smallest individual index (deterministic).
        let chosen = candidates
            .iter()
            .copied()
            .min_by(|&a, &b| {
                let da = assoc[a].1;
                let db = assoc[b].1;
                da.partial_cmp(&db)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.cmp(&b))
            })
            .expect("candidates must be non-empty");

        // Update niche count for the chosen reference point.
        let chosen_ref = assoc[chosen].0;
        niche_count[chosen_ref] += 1;

        // Remove chosen from remaining.
        remaining.retain(|&idx| idx != chosen);
        selected.push(chosen);
    }

    selected
}

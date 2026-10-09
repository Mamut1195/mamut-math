//! Fast nondominated sort — Deb 2002 (NSGA-II paper Algorithm 1).
//!
//! Complexity `O(M · N²)` where `M` is the objective count and `N` the
//! population size. Extracted verbatim from the frozen `nsga3/nondom_sort.rs`,
//! generalized from `[f64; 5]` to dynamic-`M` `&[Vec<f64>]`. The Deb-2002 loop
//! order and the insertion-order tie-break are preserved exactly (parity
//! sensitive).

use mamut_math_core::MathError;

use crate::ieee_selection::FinitePolicy;
use crate::internal;

/// Determine whether individual `p` dominates individual `q` under minimisation.
///
/// `p` dominates `q` iff `p` is no worse than `q` in all objectives and strictly
/// better in at least one. This is a low-level kernel exposed for direct reuse
/// and testing; it validates its inputs at call time.
///
/// # Errors
/// - [`ErrorKind::EmptyCollection`] when `p` or `q` is empty.
/// - [`ErrorKind::DimensionMismatch`] when `p.len() != q.len()`.
/// - [`ErrorKind::InvalidNotFinite`] when any component is `NaN`/`±Inf`.
///
/// [`ErrorKind::EmptyCollection`]: mamut_math_core::ErrorKind::EmptyCollection
/// [`ErrorKind::DimensionMismatch`]: mamut_math_core::ErrorKind::DimensionMismatch
/// [`ErrorKind::InvalidNotFinite`]: mamut_math_core::ErrorKind::InvalidNotFinite
pub fn dominates(p: &[f64], q: &[f64]) -> Result<bool, MathError> {
    use mamut_math_core::ErrorKind;
    if p.is_empty() || q.is_empty() {
        return Err(MathError::new(
            ErrorKind::EmptyCollection,
            "dominates: objective vectors must not be empty",
        ));
    }
    if p.len() != q.len() {
        return Err(MathError::new(
            ErrorKind::DimensionMismatch,
            format!("dominates: arity mismatch ({} vs {})", p.len(), q.len()),
        ));
    }
    for (index, &value) in p.iter().enumerate() {
        if !value.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("dominates: non-finite component at p[{index}]"),
            ));
        }
    }
    for (index, &value) in q.iter().enumerate() {
        if !value.is_finite() {
            return Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                format!("dominates: non-finite component at q[{index}]"),
            ));
        }
    }
    Ok(dominates_unchecked(p, q))
}

/// Unchecked dominance test over equal-arity finite slices (internal hot path).
///
/// Precondition: `p.len() == q.len()` and both slices are non-empty; the caller
/// ([`fast_nondominated_sort`]) has already validated the whole matrix. This
/// preserves the frozen Deb-2002 arithmetic and loop order exactly.
#[inline]
fn dominates_unchecked(p: &[f64], q: &[f64]) -> bool {
    let mut strictly_better = false;
    for i in 0..p.len() {
        if p[i] > q[i] {
            return false; // p is worse in at least one objective
        }
        if p[i] < q[i] {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Fast nondominated sort (Deb 2002).
///
/// Returns a vector of fronts; `fronts[i]` is the list of individual indices at
/// rank `i + 1`. Equal-objective individuals are not mutually dominating
/// (tie-break is insertion order). The concatenation of all fronts equals
/// `0..fitnesses.len()`.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when rows have `M == 0`.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when arities differ.
/// - [`mamut_math_core::ErrorKind::InvalidNotFinite`] on `NaN`/`±Inf` entries.
pub fn fast_nondominated_sort(fitnesses: &[Vec<f64>]) -> Result<Vec<Vec<usize>>, MathError> {
    fast_nondominated_sort_kernel(fitnesses, FinitePolicy::Checked)
}

/// Shared checked/raw nondominated-sort body.
///
/// The [`FinitePolicy`] toggles only the finiteness gate; the Deb-2002 loop
/// order, the insertion-order tie-break and the front concatenation are
/// identical for both routes.
pub(crate) fn fast_nondominated_sort_kernel(
    fitnesses: &[Vec<f64>],
    policy: FinitePolicy,
) -> Result<Vec<Vec<usize>>, MathError> {
    let n = fitnesses.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    let m = internal::arity(fitnesses);
    policy.validate_matrix(fitnesses, m)?;

    // domination_count[p] = number of individuals that dominate p.
    let mut domination_count: Vec<usize> = vec![0; n];
    // dominated_set[p] = indices dominated by p.
    let mut dominated_set: Vec<Vec<usize>> = vec![Vec::new(); n];

    let mut front0: Vec<usize> = Vec::new();

    for p in 0..n {
        for q in 0..n {
            if p == q {
                continue;
            }
            if dominates_unchecked(&fitnesses[p], &fitnesses[q]) {
                dominated_set[p].push(q);
            } else if dominates_unchecked(&fitnesses[q], &fitnesses[p]) {
                domination_count[p] += 1;
            }
        }
        if domination_count[p] == 0 {
            front0.push(p);
        }
    }

    let mut fronts: Vec<Vec<usize>> = Vec::new();
    let mut current_front = front0;

    while !current_front.is_empty() {
        let mut next_front: Vec<usize> = Vec::new();
        for &p in &current_front {
            for &q in &dominated_set[p] {
                domination_count[q] -= 1;
                if domination_count[q] == 0 {
                    next_front.push(q);
                }
            }
        }
        fronts.push(current_front);
        current_front = next_front;
    }

    Ok(fronts)
}

/// Indices of the first (nondominated) Pareto front.
///
/// Thin helper over [`fast_nondominated_sort`]: it returns `fronts[0]`, the
/// rank-1 front, preserving the frozen sort order. An empty fitness matrix has
/// no rank-1 front and yields `Ok(vec![])` (nothing to validate); a non-empty
/// matrix is validated exactly as by [`fast_nondominated_sort`].
///
/// # Errors
/// Propagates the validation errors of [`fast_nondominated_sort`]:
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when rows have `M == 0`.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when arities differ.
/// - [`mamut_math_core::ErrorKind::InvalidNotFinite`] on `NaN`/`±Inf` entries.
pub fn extract_pareto_front(fitnesses: &[Vec<f64>]) -> Result<Vec<usize>, MathError> {
    let fronts = fast_nondominated_sort(fitnesses)?;
    Ok(fronts.into_iter().next().unwrap_or_default())
}

/// Return whether `indices` selects a mutually nondominated subset of
/// `fitnesses` (all objectives minimized).
///
/// The subset is nondominated iff no selected individual dominates another
/// selected individual (pairwise [`dominates`] semantics, computed over the
/// validated matrix). A subset with fewer than two distinct indices is
/// trivially nondominated; a repeated index forms no pair with itself.
///
/// This is the property-test hook for front purity: the rank-1 front returned
/// by [`extract_pareto_front`] satisfies `is_nondominated_set` for every valid
/// input.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when a non-empty matrix
///   has rows with `M == 0`.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when arities differ.
/// - [`mamut_math_core::ErrorKind::InvalidNotFinite`] on `NaN`/`±Inf` entries.
/// - [`mamut_math_core::ErrorKind::IndexOutOfRange`] when an index is not
///   `< fitnesses.len()` (including any index against an empty matrix).
pub fn is_nondominated_set(fitnesses: &[Vec<f64>], indices: &[usize]) -> Result<bool, MathError> {
    use mamut_math_core::ErrorKind;

    let n = fitnesses.len();
    if n == 0 {
        if indices.is_empty() {
            return Ok(true);
        }
        return Err(MathError::new(
            ErrorKind::IndexOutOfRange,
            format!(
                "is_nondominated_set: index {} into an empty fitness matrix",
                indices[0]
            ),
        ));
    }

    let m = internal::arity(fitnesses);
    internal::validate_fitness_matrix(fitnesses, m)?;

    for &index in indices {
        if index >= n {
            return Err(MathError::new(
                ErrorKind::IndexOutOfRange,
                format!("is_nondominated_set: index {index} out of range (N = {n})"),
            ));
        }
    }

    for &a in indices {
        for &b in indices {
            if a != b && dominates_unchecked(&fitnesses[a], &fitnesses[b]) {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

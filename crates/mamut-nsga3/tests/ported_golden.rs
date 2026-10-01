//! Ported expected-value goldens from the frozen implementation's own unit
//! tests (extraction contract §D1).
//!
//! These tests re-encode the INPUT vectors and EXPECTED outputs of the frozen
//! `nsga3/*` + `operators.rs` RNG-free unit tests. Integer/count assertions are
//! bit-exact; `f64` assertions use the contract default of `1e-12` absolute
//! (the frozen `normalize` tests themselves use `1e-9` and are kept at that
//! tolerance). No frozen source file is imported: provenance is in comments.
//!
//! Frozen provenance: `git show 6db4862d7a4035de8d494cdc62607b12b4edb103`
//! (`hydro-optimizer/src/nsga3/{reference_points,nondom_sort,normalize,niching,
//! selection}.rs`, `operators.rs`).

use mamut_math_core::rng::MamutRng;
use mamut_math_core::Seed;

use mamut_nsga3::{
    adaptive_eta_value, associate, build_niche_counts, dominates, fast_nondominated_sort,
    normalize, perp_distance, select_environmental, select_from_partial_front,
    uniform_reference_points,
};

// ───────────────────────── reference_points.rs ─────────────────────────

/// Frozen `test_ref_points_count_5obj_p4`: C(8,4)=70.
#[test]
fn ref_points_count_5obj_p4_is_70() {
    let pts = uniform_reference_points(5, 4).unwrap();
    assert_eq!(pts.len(), 70);
}

/// Frozen `sum_to_one` + `test_ref_points_each_sums_to_one_5obj_p4`
/// (tolerance 1e-12, the value in the frozen code).
#[test]
fn ref_points_sum_to_one_5obj_p4() {
    let pts = uniform_reference_points(5, 4).unwrap();
    for (i, pt) in pts.iter().enumerate() {
        let s: f64 = pt.iter().sum();
        assert!((s - 1.0).abs() < 1e-12, "point[{i}] sum={s}");
    }
}

/// Frozen `test_ref_points_all_nonnegative`.
#[test]
fn ref_points_all_nonnegative() {
    let pts = uniform_reference_points(5, 4).unwrap();
    for (i, pt) in pts.iter().enumerate() {
        for (j, &v) in pt.iter().enumerate() {
            assert!(v >= 0.0, "point[{i}][{j}]={v} is negative");
        }
    }
}

/// Frozen `test_ref_points_correct_dimensionality`.
#[test]
fn ref_points_correct_dimensionality() {
    let pts = uniform_reference_points(5, 4).unwrap();
    for (i, pt) in pts.iter().enumerate() {
        assert_eq!(pt.len(), 5, "point[{i}] has wrong dimensionality");
    }
}

/// Frozen `test_ref_points_count_2obj_p3`: C(4,3)=4 and full lattice order.
#[test]
fn ref_points_count_2obj_p3_is_4() {
    let pts = uniform_reference_points(2, 3).unwrap();
    assert_eq!(pts.len(), 4);
    // gen_refs order: last coordinate forced to the remaining budget.
    assert_eq!(pts[0], vec![0.0, 1.0]);
    assert_eq!(pts[1], vec![1.0 / 3.0, 2.0 / 3.0]);
    assert_eq!(pts[2], vec![2.0 / 3.0, 1.0 / 3.0]);
    assert_eq!(pts[3], vec![1.0, 0.0]);
}

/// Frozen `test_ref_points_count_3obj_p2`: C(4,2)=6.
#[test]
fn ref_points_count_3obj_p2_is_6() {
    let pts = uniform_reference_points(3, 2).unwrap();
    assert_eq!(pts.len(), 6);
}

/// Frozen `test_ref_points_count_5obj_p1`: C(5,1)=5 corners.
#[test]
fn ref_points_count_5obj_p1_is_5() {
    let pts = uniform_reference_points(5, 1).unwrap();
    assert_eq!(pts.len(), 5);
}

/// Full 70-point lattice (5,4): exact `k/4` rationals, first/last in the frozen
/// enumeration order, all distinct.
#[test]
fn ref_points_full_lattice_5obj_p4() {
    let pts = uniform_reference_points(5, 4).unwrap();
    assert_eq!(pts.len(), 70);
    // First enumerated point: [0,0,0,0,1].
    assert_eq!(pts[0], vec![0.0, 0.0, 0.0, 0.0, 1.0]);
    // Last enumerated point: [1,0,0,0,0].
    assert_eq!(pts[69], vec![1.0, 0.0, 0.0, 0.0, 0.0]);
    // Every coordinate is an exact multiple of 1/4.
    for pt in &pts {
        for &v in pt {
            assert!(
                ((v * 4.0) - (v * 4.0).round()).abs() < 1e-12,
                "{v} is not k/4"
            );
        }
    }
    // All 70 points are distinct.
    let mut sorted = pts.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted.dedup();
    assert_eq!(sorted.len(), 70);
}

// ───────────────────────── nondom_sort.rs ─────────────────────────

/// Frozen `dominates_strict`.
#[test]
fn dominates_strict() {
    let a = vec![1.0_f64; 5];
    let b = vec![2.0_f64; 5];
    assert!(dominates(&a, &b).unwrap());
    assert!(!dominates(&b, &a).unwrap());
}

/// Frozen `dominates_equal_is_not_domination`.
#[test]
fn dominates_equal_is_not_domination() {
    let a = vec![1.0_f64; 5];
    assert!(!dominates(&a, &a).unwrap());
}

/// Frozen `dominates_partial_tradeoff`.
#[test]
fn dominates_partial_tradeoff() {
    let a = vec![1.0, 2.0, 0.0, 0.0, 0.0];
    let b = vec![2.0, 1.0, 0.0, 0.0, 0.0];
    assert!(!dominates(&a, &b).unwrap());
    assert!(!dominates(&b, &a).unwrap());
}

/// Frozen `test_nondom_sort_known_3point_2obj`.
#[test]
fn nondom_sort_known_3point_2obj() {
    let fitnesses = vec![
        vec![1.0, 2.0, 0.0, 0.0, 0.0],
        vec![2.0, 1.0, 0.0, 0.0, 0.0],
        vec![3.0, 3.0, 0.0, 0.0, 0.0],
    ];
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    assert_eq!(fronts.len(), 2);
    let mut f0 = fronts[0].clone();
    let mut f1 = fronts[1].clone();
    f0.sort_unstable();
    f1.sort_unstable();
    assert_eq!(f0, vec![0, 1]);
    assert_eq!(f1, vec![2]);
}

/// Frozen `test_nondom_sort_all_identical_objectives` — exact insertion order.
#[test]
fn nondom_sort_all_identical_objectives_preserves_insertion_order() {
    let fitnesses = vec![vec![1.0; 5], vec![1.0; 5], vec![1.0; 5], vec![1.0; 5]];
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    assert_eq!(fronts.len(), 1);
    assert_eq!(fronts[0], vec![0, 1, 2, 3]);
}

/// Frozen `test_nondom_sort_strictly_dominated_chain`.
#[test]
fn nondom_sort_strictly_dominated_chain() {
    let fitnesses = vec![vec![1.0; 5], vec![2.0; 5], vec![3.0; 5]];
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    assert_eq!(fronts.len(), 3);
    assert_eq!(fronts[0], vec![0]);
    assert_eq!(fronts[1], vec![1]);
    assert_eq!(fronts[2], vec![2]);
}

/// Frozen `test_nondom_sort_single_individual`.
#[test]
fn nondom_sort_single_individual() {
    let fitnesses = vec![vec![5.0, 3.0, 1.0, 2.0, 4.0]];
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    assert_eq!(fronts.len(), 1);
    assert_eq!(fronts[0], vec![0]);
}

/// Frozen `test_nondom_sort_empty_population`.
#[test]
fn nondom_sort_empty_population() {
    let fronts = fast_nondominated_sort(&[]).unwrap();
    assert!(fronts.is_empty());
}

/// Frozen `test_nondom_sort_all_indices_covered`.
#[test]
fn nondom_sort_all_indices_covered() {
    let fitnesses = vec![
        vec![1.0, 5.0, 0.0, 0.0, 0.0],
        vec![3.0, 3.0, 0.0, 0.0, 0.0],
        vec![5.0, 1.0, 0.0, 0.0, 0.0],
        vec![4.0, 4.0, 0.0, 0.0, 0.0],
        vec![2.0, 6.0, 0.0, 0.0, 0.0],
    ];
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    let mut all: Vec<usize> = fronts.iter().flatten().copied().collect();
    all.sort_unstable();
    assert_eq!(all, vec![0, 1, 2, 3, 4]);
}

// ───────────────────────── normalize.rs ─────────────────────────

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

/// Frozen `test_normalize_ideal_is_per_obj_minimum`.
#[test]
fn normalize_ideal_is_per_obj_minimum() {
    let fitnesses = vec![
        vec![3.0, 1.0, 4.0, 1.0, 5.0],
        vec![1.0, 5.0, 2.0, 3.0, 2.0],
        vec![2.0, 3.0, 1.0, 5.0, 1.0],
    ];
    let result = normalize(&fitnesses).unwrap();
    assert_eq!(result.ideal, vec![1.0, 1.0, 1.0, 1.0, 1.0]);
}

/// Frozen `test_normalize_translated_nonnegative`.
#[test]
fn normalize_translated_nonnegative() {
    let fitnesses = vec![
        vec![10.0, 5.0, 2.0, 8.0, 3.0],
        vec![2.0, 8.0, 5.0, 3.0, 7.0],
        vec![6.0, 3.0, 9.0, 1.0, 4.0],
    ];
    let result = normalize(&fitnesses).unwrap();
    for row in &result.normalized {
        for &v in row {
            assert!(v >= -1e-12, "normalized value {v} is negative");
        }
    }
}

/// Frozen `test_normalize_intercepts_positive_for_well_conditioned_input`.
#[test]
fn normalize_intercepts_positive_for_well_conditioned_input() {
    let fitnesses = vec![
        vec![5.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 4.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 3.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 2.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 6.0],
    ];
    let result = normalize(&fitnesses).unwrap();
    for (j, &ic) in result.intercepts.iter().enumerate() {
        assert!(ic > 0.0, "intercept[{j}] = {ic} must be positive");
    }
}

/// Frozen `test_normalize_known_corner_values` (tolerance 1e-9).
#[test]
fn normalize_known_corner_values() {
    let (a, b, c, d, e) = (2.0_f64, 3.0, 4.0, 5.0, 6.0);
    let fitnesses = vec![
        vec![1.0 + a, 1.0, 1.0, 1.0, 1.0],
        vec![1.0, 1.0 + b, 1.0, 1.0, 1.0],
        vec![1.0, 1.0, 1.0 + c, 1.0, 1.0],
        vec![1.0, 1.0, 1.0, 1.0 + d, 1.0],
        vec![1.0, 1.0, 1.0, 1.0, 1.0 + e],
    ];
    let result = normalize(&fitnesses).unwrap();
    let tol = 1e-9;
    assert!(approx_eq(result.intercepts[0], a, tol));
    assert!(approx_eq(result.intercepts[1], b, tol));
    assert!(approx_eq(result.intercepts[2], c, tol));
    assert!(approx_eq(result.intercepts[3], d, tol));
    assert!(approx_eq(result.intercepts[4], e, tol));
}

/// Frozen `test_normalize_degenerate_all_identical_no_panic`.
#[test]
fn normalize_degenerate_all_identical_no_panic() {
    let fitnesses = vec![vec![2.0; 5], vec![2.0; 5], vec![2.0; 5]];
    let result = normalize(&fitnesses).unwrap();
    for &ic in &result.intercepts {
        assert!(ic > 0.0, "degenerate fallback intercept must be positive");
    }
}

/// Frozen `test_normalize_empty_input`.
///
/// Deviation (documented): the frozen fixed `[f64; 5]` code returns
/// `ideal = [0;5]` / `intercepts = [1;5]`; with an empty input the dynamic-`M`
/// crate cannot infer `M`, so all three vectors are zero-length.
#[test]
fn normalize_empty_input() {
    let result = normalize(&[]).unwrap();
    assert!(result.normalized.is_empty());
    assert!(result.ideal.is_empty());
    assert!(result.intercepts.is_empty());
}

/// Frozen `test_normalize_corner_individual_maps_to_unit_value`.
#[test]
fn normalize_corner_individual_maps_to_unit_value() {
    let fitnesses = vec![
        vec![1.0 + 5.0, 1.0, 1.0, 1.0, 1.0],
        vec![1.0, 1.0 + 4.0, 1.0, 1.0, 1.0],
        vec![1.0, 1.0, 1.0 + 3.0, 1.0, 1.0],
        vec![1.0, 1.0, 1.0, 1.0 + 2.0, 1.0],
        vec![1.0, 1.0, 1.0, 1.0, 1.0 + 6.0],
    ];
    let result = normalize(&fitnesses).unwrap();
    assert!(approx_eq(result.normalized[0][0], 1.0, 1e-9));
    assert!(approx_eq(result.normalized[0][1], 0.0, 1e-9));
}

// ───────────────────────── niching.rs ─────────────────────────

/// Frozen `test_perp_distance_aligned_is_zero`.
#[test]
fn perp_distance_aligned_is_zero() {
    let p = vec![1.0_f64, 0.0, 0.0, 0.0, 0.0];
    let r = vec![1.0_f64, 0.0, 0.0, 0.0, 0.0];
    let d = perp_distance(&p, &r).unwrap();
    assert!(d < 1e-10, "aligned point must have distance 0, got {d}");
}

/// Frozen `test_perp_distance_orthogonal_refs`.
#[test]
fn perp_distance_orthogonal_refs() {
    let p = vec![1.0_f64, 0.0, 0.0, 0.0, 0.0];
    let r = vec![0.0_f64, 1.0, 0.0, 0.0, 0.0];
    let d = perp_distance(&p, &r).unwrap();
    assert!((d - 1.0).abs() < 1e-10, "distance must be 1.0, got {d}");
}

/// Frozen `test_perp_distance_diagonal_aligned`.
#[test]
fn perp_distance_diagonal_aligned() {
    let p = vec![1.0_f64, 1.0, 0.0, 0.0, 0.0];
    let r = vec![1.0_f64, 1.0, 0.0, 0.0, 0.0];
    let d = perp_distance(&p, &r).unwrap();
    assert!(d < 1e-10, "aligned diagonal must have d=0, got {d}");
}

/// Frozen `test_associate_picks_nearest_ref`.
#[test]
fn associate_picks_nearest_ref() {
    let normalized = vec![vec![0.9, 0.1, 0.0, 0.0, 0.0], vec![0.1, 0.9, 0.0, 0.0, 0.0]];
    let refs = vec![vec![1.0, 0.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0, 0.0]];
    let assoc = associate(&normalized, &refs).unwrap();
    assert_eq!(assoc[0].0, 0);
    assert_eq!(assoc[1].0, 1);
}

/// Frozen `test_associate_single_ref_all_map_to_zero`.
#[test]
fn associate_single_ref_all_map_to_zero() {
    let normalized = vec![
        vec![1.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 1.0],
        vec![0.5, 0.5, 0.0, 0.0, 0.0],
    ];
    let refs = vec![vec![0.2, 0.2, 0.2, 0.2, 0.2]];
    let assoc = associate(&normalized, &refs).unwrap();
    for (i, &(ref_idx, _)) in assoc.iter().enumerate() {
        assert_eq!(ref_idx, 0, "individual {i} must map to ref 0");
    }
}

/// Frozen `test_build_niche_counts_basic`.
#[test]
fn build_niche_counts_basic() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.1), (0, 0.2), (1, 0.3)];
    let counts = build_niche_counts(&[0, 1, 2], &assoc, 2);
    assert_eq!(counts[0], 2);
    assert_eq!(counts[1], 1);
}

/// Frozen `test_build_niche_counts_empty_subset`.
#[test]
fn build_niche_counts_empty_subset() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.1), (1, 0.2)];
    let counts = build_niche_counts(&[], &assoc, 2);
    assert_eq!(counts, vec![0, 0]);
}

/// Frozen `test_select_from_partial_front_prefers_lowest_niche`.
#[test]
fn select_from_partial_front_prefers_lowest_niche() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.5), (1, 0.5)];
    let mut niche_count = vec![0_u32, 1_u32];
    let selected = select_from_partial_front(&[0, 1], &assoc, &mut niche_count, 1);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0], 0);
}

/// Frozen `test_select_from_partial_front_tie_breaks_by_distance`.
#[test]
fn select_from_partial_front_tie_breaks_by_distance() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.8), (0, 0.2)];
    let mut niche_count = vec![0_u32];
    let selected = select_from_partial_front(&[0, 1], &assoc, &mut niche_count, 1);
    assert_eq!(selected[0], 1);
}

/// Frozen `test_select_from_partial_front_selects_all_if_needed_equals_len`.
#[test]
fn select_from_partial_front_selects_all_if_needed_equals_len() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.1), (1, 0.2), (0, 0.3)];
    let mut niche_count = vec![0_u32, 0_u32];
    let selected = select_from_partial_front(&[0, 1, 2], &assoc, &mut niche_count, 3);
    assert_eq!(selected.len(), 3);
}

/// Frozen `test_select_from_partial_front_niche_count_incremented`.
#[test]
fn select_from_partial_front_niche_count_incremented() {
    let assoc: Vec<(usize, f64)> = vec![(0, 0.1), (0, 0.2)];
    let mut niche_count = vec![0_u32];
    let _ = select_from_partial_front(&[0, 1], &assoc, &mut niche_count, 2);
    assert_eq!(niche_count[0], 2);
}

// ───────────────────────── selection.rs ─────────────────────────

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

fn make_rng(seed: u64) -> MamutRng {
    MamutRng::root(&Seed::from_u64(seed))
}

/// Frozen `test_select_environmental_returns_n`.
#[test]
fn select_environmental_returns_n() {
    let fitnesses = synthetic_fitnesses(20);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 10, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 10);
}

/// Frozen `test_select_environmental_returns_n_large`.
#[test]
fn select_environmental_returns_n_large() {
    let fitnesses = synthetic_fitnesses(200);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 100, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 100);
}

/// Frozen `test_select_environmental_all_returned_when_pop_le_n`.
#[test]
fn select_environmental_all_returned_when_pop_le_n() {
    let fitnesses = synthetic_fitnesses(5);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 10, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 5);
}

/// Frozen `test_select_environmental_valid_indices`.
#[test]
fn select_environmental_valid_indices() {
    let n_pop = 50;
    let fitnesses = synthetic_fitnesses(n_pop);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 25, &refs, &mut rng).unwrap();
    for &idx in &selected {
        assert!(idx < n_pop);
    }
}

/// Frozen `test_select_environmental_no_duplicates`.
#[test]
fn select_environmental_no_duplicates() {
    let fitnesses = synthetic_fitnesses(40);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 20, &refs, &mut rng).unwrap();
    let unique: std::collections::BTreeSet<usize> = selected.iter().copied().collect();
    assert_eq!(unique.len(), selected.len());
}

/// Frozen `test_select_environmental_empty_population`.
#[test]
fn select_environmental_empty_population() {
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&[], 10, &refs, &mut rng).unwrap();
    assert!(selected.is_empty());
}

/// Frozen `test_select_environmental_n_zero`.
#[test]
fn select_environmental_n_zero() {
    let fitnesses = synthetic_fitnesses(10);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 0, &refs, &mut rng).unwrap();
    assert!(selected.is_empty());
}

/// Frozen `test_select_environmental_lower_niche_preferred`.
#[test]
fn select_environmental_lower_niche_preferred() {
    let fitnesses = vec![
        vec![1.0, 2.0, 0.0, 0.0, 0.0],
        vec![2.0, 1.0, 0.0, 0.0, 0.0],
        vec![3.0, 3.0, 0.0, 0.0, 0.0],
        vec![4.0, 4.0, 0.0, 0.0, 0.0],
    ];
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng = make_rng(0);
    let selected = select_environmental(&fitnesses, 3, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 3);
    let set: std::collections::BTreeSet<usize> = selected.iter().copied().collect();
    assert!(set.contains(&0));
    assert!(set.contains(&1));
}

/// Frozen `test_select_environmental_deterministic` (seed 123 twice).
#[test]
fn select_environmental_deterministic() {
    let fitnesses = synthetic_fitnesses(30);
    let refs = uniform_reference_points(5, 4).unwrap();
    let mut rng1 = make_rng(123);
    let sel1 = select_environmental(&fitnesses, 15, &refs, &mut rng1).unwrap();
    let mut rng2 = make_rng(123);
    let sel2 = select_environmental(&fitnesses, 15, &refs, &mut rng2).unwrap();
    assert_eq!(sel1, sel2);
}

// ───────────────────────── operators.rs (RNG-free) ─────────────────────────

/// Frozen `test_adaptive_eta_at_gen_0_equals_eta_min`.
#[test]
fn adaptive_eta_at_gen_0_equals_eta_min() {
    let result = adaptive_eta_value(0, 100, 5.0, 50.0);
    assert!((result - 5.0).abs() < 1e-12, "expected 5.0, got {result}");
}

/// Frozen `test_adaptive_eta_at_gen_max_equals_eta_max`.
#[test]
fn adaptive_eta_at_gen_max_equals_eta_max() {
    let result = adaptive_eta_value(100, 100, 5.0, 50.0);
    assert!((result - 50.0).abs() < 1e-12, "expected 50.0, got {result}");
}

/// Frozen `test_adaptive_eta_at_midpoint_equals_midpoint_value`.
#[test]
fn adaptive_eta_at_midpoint_equals_midpoint_value() {
    let result = adaptive_eta_value(50, 100, 5.0, 50.0);
    assert!((result - 27.5).abs() < 1e-12, "expected 27.5, got {result}");
}

/// Frozen `test_adaptive_eta_max_gen_zero_does_not_panic`.
#[test]
fn adaptive_eta_max_gen_zero_does_not_panic() {
    let result = adaptive_eta_value(0, 0, 5.0, 50.0);
    assert!(result.is_finite());
}

/// Frozen `test_adaptive_eta_monotone_increasing`.
#[test]
fn adaptive_eta_monotone_increasing() {
    let etas: Vec<f64> = (0..=10)
        .map(|g| adaptive_eta_value(g, 10, 5.0, 50.0))
        .collect();
    for w in etas.windows(2) {
        assert!(w[1] >= w[0]);
    }
}

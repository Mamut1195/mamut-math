//! Property / invariant tests for the NSGA-III kernels, including the binding
//! dynamic-`M` validation contract (H2): `EmptyCollection` for `M == 0`,
//! `DimensionMismatch` for ragged/mismatched input, `InvalidNotFinite` for
//! non-finite entries.

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{ErrorKind, Seed, ValidFloat, Variable, VariableId, VariableKind};
use mamut_nsga3::{
    associate, build_niche_counts, extract_pareto_front, fast_nondominated_sort, hypervolume,
    igd_plus, init_population, is_nondominated_set, normalize, perp_distance, polynomial_mutation,
    sbx_crossover, select_environmental, uniform_reference_points, var_or, Individual,
};

fn variables(bounds: &[(f64, f64)]) -> Vec<Variable> {
    bounds
        .iter()
        .enumerate()
        .map(|(i, &(lb, ub))| {
            Variable::new(
                VariableId(i as u64),
                VariableKind::continuous(
                    ValidFloat::new(lb).unwrap(),
                    ValidFloat::new(ub).unwrap(),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect()
}

// ───────────────────────── H2: dynamic-M validation ─────────────────────────

#[test]
fn m_zero_is_empty_collection() {
    assert_eq!(
        fast_nondominated_sort(&[vec![]]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        normalize(&[vec![]]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn ragged_input_is_dimension_mismatch() {
    let ragged = vec![vec![1.0, 2.0], vec![1.0, 2.0, 3.0]];
    assert_eq!(
        fast_nondominated_sort(&ragged).unwrap_err().kind(),
        ErrorKind::DimensionMismatch
    );
    assert_eq!(
        normalize(&ragged).unwrap_err().kind(),
        ErrorKind::DimensionMismatch
    );
}

#[test]
fn dominates_validates_inputs() {
    use mamut_nsga3::dominates;

    // Empty objective vectors -> EmptyCollection.
    assert_eq!(
        dominates(&[], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        dominates(&[1.0], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    // Arity mismatch -> DimensionMismatch.
    assert_eq!(
        dominates(&[1.0, 2.0], &[1.0]).unwrap_err().kind(),
        ErrorKind::DimensionMismatch
    );
    // Non-finite components -> InvalidNotFinite (either operand).
    assert_eq!(
        dominates(&[f64::NAN], &[1.0]).unwrap_err().kind(),
        ErrorKind::InvalidNotFinite
    );
    assert_eq!(
        dominates(&[1.0], &[f64::INFINITY]).unwrap_err().kind(),
        ErrorKind::InvalidNotFinite
    );
    // Valid-input dominance semantics are preserved.
    assert!(dominates(&[1.0, 1.0], &[2.0, 2.0]).unwrap());
    assert!(!dominates(&[2.0, 2.0], &[1.0, 1.0]).unwrap());
    assert!(!dominates(&[1.0, 2.0], &[2.0, 1.0]).unwrap());
    assert!(!dominates(&[1.0, 1.0], &[1.0, 1.0]).unwrap());
}

#[test]
fn non_finite_input_is_invalid_not_finite() {
    assert_eq!(
        fast_nondominated_sort(&[vec![f64::NAN, 1.0]])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    assert_eq!(
        normalize(&[vec![f64::INFINITY, 1.0]]).unwrap_err().kind(),
        ErrorKind::InvalidNotFinite
    );

    let mut rng = MamutRng::root(&Seed::from_u64(0));
    let refs = uniform_reference_points(2, 1).unwrap();
    assert_eq!(
        select_environmental(&[vec![1.0, 1.0], vec![f64::NAN, 0.0]], 1, &refs, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
}

#[test]
fn reference_point_validation() {
    assert_eq!(
        uniform_reference_points(0, 4).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        uniform_reference_points(5, 0).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn associate_rejects_empty_refs_and_ragged_refs() {
    let normalized = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    assert_eq!(
        associate(&normalized, &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        associate(&normalized, &[vec![1.0, 0.0, 0.0]])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
}

#[test]
fn perp_distance_rejects_arity_mismatch() {
    assert_eq!(
        perp_distance(&[1.0, 2.0], &[1.0]).unwrap_err().kind(),
        ErrorKind::DimensionMismatch
    );
    assert_eq!(
        perp_distance(&[], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn perp_distance_rejects_non_finite() {
    // Non-finite point component.
    assert_eq!(
        perp_distance(&[f64::NAN, 1.0], &[1.0, 1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    // Non-finite reference component.
    assert_eq!(
        perp_distance(&[1.0, 1.0], &[1.0, f64::INFINITY])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    // Finite inputs still use the unchanged degenerate 1e-30 guard (zero ref).
    let d = perp_distance(&[1.0, 0.0], &[0.0, 0.0]).unwrap();
    assert!((d - 1.0).abs() < 1e-12);
}

// ───────────────────────── reference points ─────────────────────────

fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    (0..k).fold(1usize, |acc, i| acc * (n - i) / (i + 1))
}

#[test]
fn reference_point_count_is_binomial_lattice() {
    for &(nobj, p) in &[(2usize, 3usize), (3, 2), (5, 1), (4, 3), (5, 4), (6, 5)] {
        let pts = uniform_reference_points(nobj, p).unwrap();
        assert_eq!(pts.len(), binomial(nobj + p - 1, p), "({nobj},{p})");
        for pt in &pts {
            assert_eq!(pt.len(), nobj);
            assert!((pt.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            assert!(pt.iter().all(|&v| v >= 0.0));
        }
    }
}

// ───────────────────────── nondominated sort ─────────────────────────

#[test]
fn nondom_sort_covers_every_index_and_fronts_are_antichains() {
    let fitnesses: Vec<Vec<f64>> = (0..25)
        .map(|i| {
            let t = i as f64 / 25.0;
            vec![t, 1.0 - t, (t - 0.5).abs(), 1.0 - (t - 0.5).abs()]
        })
        .collect();
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();

    let mut all: Vec<usize> = fronts.iter().flatten().copied().collect();
    all.sort_unstable();
    assert_eq!(all, (0..25).collect::<Vec<_>>());

    // No individual in a front dominates another in the same front.
    for front in &fronts {
        for &a in front {
            for &b in front {
                if a != b {
                    assert!(!mamut_nsga3::dominates(&fitnesses[a], &fitnesses[b]).unwrap());
                }
            }
        }
    }
}

// ───────────────────────── normalization / niching ─────────────────────────

#[test]
fn normalize_produces_nonnegative_and_positive_intercepts() {
    let fitnesses = vec![
        vec![5.0, 0.0, 0.0],
        vec![0.0, 4.0, 0.0],
        vec![0.0, 0.0, 3.0],
        vec![2.0, 2.0, 2.0],
    ];
    let result = normalize(&fitnesses).unwrap();
    assert!(result.intercepts.iter().all(|&ic| ic > 0.0));
    for row in &result.normalized {
        assert!(row.iter().all(|&v| v >= -1e-12));
    }
}

#[test]
fn niche_counts_sum_to_subset_size() {
    let assoc = vec![(0usize, 0.1), (2, 0.2), (1, 0.3), (0, 0.4)];
    let counts = build_niche_counts(&[0, 1, 3], &assoc, 3);
    assert_eq!(counts.iter().sum::<u32>(), 3);
}

// ───────────────────────── selection ─────────────────────────

#[test]
fn select_environmental_is_a_set_of_min_size() {
    let refs = uniform_reference_points(5, 4).unwrap();
    let fitnesses: Vec<Vec<f64>> = (0..40)
        .map(|i| {
            let t = i as f64 / 40.0;
            vec![t, 1.0 - t, 0.5 * t, 1.0 - 0.5 * t, 0.25]
        })
        .collect();
    let mut rng = MamutRng::root(&Seed::from_u64(0));
    let selected = select_environmental(&fitnesses, 15, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 15);
    let unique: std::collections::BTreeSet<usize> = selected.iter().copied().collect();
    assert_eq!(unique.len(), 15);
    assert!(selected.iter().all(|&i| i < 40));
}

#[test]
fn select_environmental_is_deterministic_under_mamut_rng() {
    let refs = uniform_reference_points(5, 4).unwrap();
    let fitnesses: Vec<Vec<f64>> = (0..30)
        .map(|i| {
            let t = i as f64 / 30.0;
            vec![t, 1.0 - t, 0.5, 0.5, 0.5]
        })
        .collect();
    let mut a = MamutRng::root(&Seed::from_u64(123));
    let mut b = MamutRng::root(&Seed::from_u64(123));
    assert_eq!(
        select_environmental(&fitnesses, 12, &refs, &mut a).unwrap(),
        select_environmental(&fitnesses, 12, &refs, &mut b).unwrap()
    );
}

// ───────────────────────── operators ─────────────────────────

#[test]
fn sbx_and_mutation_stay_within_bounds_over_a_sweep() {
    let bounds = [
        (-2.0, 3.0),
        (0.0, 1.0),
        (1.0, 1.5),
        (-5.0, 5.0),
        (60.0, 120.0),
    ];
    let vars = variables(&bounds);
    let mut rng = MamutRng::root(&Seed::from_u64(42));

    for _ in 0..200 {
        let mut p1 = Individual::new(
            mamut_math_core::Candidate::new(
                bounds
                    .iter()
                    .map(|&(lb, ub)| ValidFloat::new(0.5 * (lb + ub)).unwrap())
                    .collect(),
            )
            .unwrap(),
        );
        let mut p2 = Individual::new(
            mamut_math_core::Candidate::new(
                bounds
                    .iter()
                    .map(|&(lb, ub)| ValidFloat::new(lb + 0.9 * (ub - lb)).unwrap())
                    .collect(),
            )
            .unwrap(),
        );
        sbx_crossover(&mut p1, &mut p2, &vars, 15.0, &mut rng).unwrap();
        for (g, &(lb, ub)) in p1.genes().iter().zip(bounds.iter()) {
            assert!(*g >= lb && *g <= ub);
        }

        let mut ind = p1.clone();
        polynomial_mutation(&mut ind, &vars, 20.0, &mut rng).unwrap();
        for (g, &(lb, ub)) in ind.genes().iter().zip(bounds.iter()) {
            assert!(*g >= lb && *g <= ub);
        }
    }
}

#[test]
fn var_or_returns_lambda_and_clears_fitness_only_when_altered() {
    let bounds = [(-2.0, 3.0), (0.0, 1.0), (1.0, 1.5)];
    let vars = variables(&bounds);
    let genes = [0.5, 0.5, 1.2];
    let mut population = Vec::new();
    for k in 0..4 {
        let g = [genes[0] + 0.1 * k as f64, genes[1], genes[2]];
        let candidate = mamut_math_core::Candidate::new(
            g.iter().map(|&v| ValidFloat::new(v).unwrap()).collect(),
        )
        .unwrap();
        population.push(Individual::evaluated(
            candidate,
            mamut_math_core::ObjectiveValues::new(vec![
                ValidFloat::new(1.0).unwrap(),
                ValidFloat::new(1.0).unwrap(),
            ])
            .unwrap(),
            mamut_math_core::Feasibility::Feasible,
        ));
    }

    let mut rng = MamutRng::root(&Seed::from_u64(7));
    let offspring = var_or(&population, &vars, 20, 0.7, 0.2, 15.0, &mut rng).unwrap();
    assert_eq!(offspring.len(), 20);
    for child in &offspring {
        for (g, &(lb, ub)) in child.genes().iter().zip(bounds.iter()) {
            assert!(*g >= lb && *g <= ub);
        }
    }
    // Pure reproduction (cxpb = mutpb = 0) yields exact clones with fitness.
    let mut rng = MamutRng::root(&Seed::from_u64(0));
    let clones = var_or(&population, &vars, 5, 0.0, 0.0, 15.0, &mut rng).unwrap();
    for clone in &clones {
        assert!(
            clone.fitness.is_some(),
            "reproduction must keep cached fitness"
        );
    }
}

#[test]
fn var_or_rejects_invalid_probabilities_and_empty_population() {
    let vars = variables(&[(0.0, 1.0)]);
    let mut rng = MamutRng::root(&Seed::from_u64(0));
    let population = init_population(&vars, 2, &mut rng).unwrap();
    assert_eq!(
        var_or(&population, &vars, 2, 0.8, 0.5, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::InvertedBounds
    );
    assert_eq!(
        var_or(&[], &vars, 2, 0.5, 0.5, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn init_population_is_deterministic_and_unevaluated() {
    let vars = variables(&[(0.0, 1.0), (-1.0, 1.0)]);
    let run = |seed: u64| {
        let mut rng = MamutRng::root(&Seed::from_u64(seed));
        init_population(&vars, 10, &mut rng)
            .unwrap()
            .iter()
            .map(Individual::genes)
            .collect::<Vec<_>>()
    };
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(43));

    let mut rng = MamutRng::root(&Seed::from_u64(1));
    for ind in init_population(&vars, 10, &mut rng).unwrap() {
        assert!(!ind.is_evaluated());
        assert!(ind.fitness.is_none());
    }
}

// ───────────────────────── Pareto metrics ─────────────────────────

fn sample_population(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![
                t,
                1.0 - t,
                (t - 0.5).abs(),
                1.0 - (t - 0.5).abs(),
                0.25 + 0.5 * t,
            ]
        })
        .collect()
}

#[test]
fn extract_pareto_front_is_pure_and_matches_first_front() {
    let fitnesses = sample_population(31);
    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    let front = extract_pareto_front(&fitnesses).unwrap();

    // Thin helper: exactly the rank-1 front, in the frozen sort order.
    assert_eq!(front, fronts[0]);

    // Internal antichain property through the public validator hook.
    assert!(is_nondominated_set(&fitnesses, &front).unwrap());

    // Front purity: no extracted point is dominated by any point of the whole
    // population (not merely by another point of the front).
    for &i in &front {
        for j in 0..fitnesses.len() {
            if i != j {
                assert!(!mamut_nsga3::dominates(&fitnesses[j], &fitnesses[i]).unwrap());
            }
        }
    }
}

#[test]
fn extract_pareto_front_empty_input_is_empty() {
    assert!(extract_pareto_front(&[]).unwrap().is_empty());
    assert!(is_nondominated_set(&[], &[]).unwrap());
}

/// Focused, non-vacuous front-extraction test: the population provably
/// contains dominated candidates, so the assertions genuinely exercise
/// dominated-point exclusion (unlike the trade-off-only sweep above).
#[test]
fn extract_pareto_front_excludes_dominated_points_non_vacuously() {
    // 2-objective minimization population with known dominated candidates.
    let fitnesses = vec![
        vec![1.0, 5.0], // 0: nondominated
        vec![3.0, 3.0], // 1: nondominated
        vec![5.0, 1.0], // 2: nondominated
        vec![4.0, 4.0], // 3: dominated by 1
        vec![3.0, 3.0], // 4: duplicate of 1 (tie -> nondominated)
        vec![6.0, 6.0], // 5: dominated by multiple others
        vec![9.0, 2.0], // 6: dominated by 2
        vec![2.0, 8.0], // 7: dominated by 0
    ];
    let n = fitnesses.len();

    // (d) Non-vacuity premise computed in-test: some index is dominated by at
    // least one other, and index 5 has multiple distinct dominators.
    let dominated: Vec<usize> = (0..n)
        .filter(|&i| {
            (0..n).any(|j| j != i && mamut_nsga3::dominates(&fitnesses[j], &fitnesses[i]).unwrap())
        })
        .collect();
    assert_eq!(
        dominated,
        vec![3, 5, 6, 7],
        "test premise: dominated indices"
    );
    let dominators_of_5 = (0..n)
        .filter(|&j| j != 5 && mamut_nsga3::dominates(&fitnesses[j], &fitnesses[5]).unwrap())
        .count();
    assert!(
        dominators_of_5 >= 2,
        "index 5 must be dominated by multiple points, found {dominators_of_5}"
    );

    // (a) Exact true nondominated set, in the frozen insertion order, and equal
    // to the rank-1 front of the full sort.
    let front = extract_pareto_front(&fitnesses).unwrap();
    assert_eq!(front, vec![0, 1, 2, 4]);
    assert_eq!(front, fast_nondominated_sort(&fitnesses).unwrap()[0]);

    // (b) No dominated index is present in the extracted front.
    for &index in &dominated {
        assert!(
            !front.contains(&index),
            "dominated index {index} was extracted"
        );
    }

    // (c) The extracted front is internally nondominated, while the full
    // population is not.
    let all_indices: Vec<usize> = (0..n).collect();
    assert!(is_nondominated_set(&fitnesses, &front).unwrap());
    assert!(!is_nondominated_set(&fitnesses, &all_indices).unwrap());
}

#[test]
fn is_nondominated_set_detects_a_dominated_subset() {
    let fitnesses = vec![
        vec![1.0, 1.0],
        vec![2.0, 2.0], // dominated by index 0
        vec![0.5, 3.0], // incomparable with index 0
    ];
    assert!(is_nondominated_set(&fitnesses, &[0, 2]).unwrap());
    assert!(!is_nondominated_set(&fitnesses, &[0, 1]).unwrap());
    assert!(is_nondominated_set(&fitnesses, &[1]).unwrap());
    assert_eq!(
        is_nondominated_set(&fitnesses, &[3]).unwrap_err().kind(),
        ErrorKind::IndexOutOfRange
    );
}

#[test]
fn hypervolume_never_decreases_when_a_point_is_added() {
    let reference = [6.0, 6.0, 6.0];
    let base = vec![
        vec![1.0, 3.0, 2.0],
        vec![3.0, 1.0, 2.0],
        vec![2.0, 2.0, 1.0],
    ];
    let base_hv = hypervolume(&base, &reference).unwrap();

    // A genuinely nondominated point extends the front; the union measure can
    // only grow.
    let mut extended = base.clone();
    extended.push(vec![0.5, 4.0, 4.0]);
    let extended_hv = hypervolume(&extended, &reference).unwrap();
    assert!(extended_hv + 1e-12 >= base_hv);

    // A dominated point adds no volume at all.
    let mut dominated = base.clone();
    dominated.push(vec![3.5, 3.5, 3.0]);
    let dominated_hv = hypervolume(&dominated, &reference).unwrap();
    assert!((dominated_hv - base_hv).abs() <= 1e-12);
}

#[test]
fn igd_plus_identity_property_holds_for_sample_fronts() {
    for n in [1usize, 2, 5, 17] {
        let front = sample_population(n);
        assert_eq!(igd_plus(&front, &front).unwrap(), 0.0, "N = {n}");
    }
}

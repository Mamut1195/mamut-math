//! Property / invariant tests for the NSGA-III kernels, including the binding
//! dynamic-`M` validation contract (H2): `EmptyCollection` for `M == 0`,
//! `DimensionMismatch` for ragged/mismatched input, `InvalidNotFinite` for
//! non-finite entries.

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{ErrorKind, Seed, ValidFloat, Variable, VariableId, VariableKind};
use mamut_nsga3::{
    associate, build_niche_counts, fast_nondominated_sort, init_population, normalize,
    perp_distance, polynomial_mutation, sbx_crossover, select_environmental,
    uniform_reference_points, var_or, Individual,
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

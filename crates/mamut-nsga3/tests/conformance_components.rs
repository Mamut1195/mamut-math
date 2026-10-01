//! Component-level NSGA-III conformance against an independent oracle.
//!
//! Consumes the committed `component_oracle.json` (generated offline by
//! `scripts/oracles/component_oracle.py` from pymoo + DEAP) and feeds the SAME
//! inputs to the crate kernels:
//!
//! - `uniform_reference_points` vs pymoo das-dennis lattices,
//! - `fast_nondominated_sort` vs DEAP `sortNondominated`,
//! - `normalize` (ideal / intercepts / normalized) vs pymoo
//!   `find_ideal` + `get_extreme_points_c` + the pymoo intercept solve,
//! - `hypervolume` vs DEAP's exact 3D compiled hypervolume,
//! - `igd_plus` vs an in-script numpy IGD+,
//! - `sbx_crossover` / `polynomial_mutation` vs DEAP operators driven by the
//!   committed uniform stream.
//!
//! Tolerances: index / enumeration outputs are exact; `f64` metric outputs are
//! `<= 1e-9` absolute (intercepts `<= 1e-9`, with the documented loop-order
//! caveat that the Rust kernel accumulates translated values in a different
//! order than numpy).

mod support;

use mamut_math_core::{Candidate, ValidFloat, Variable, VariableId, VariableKind};
use mamut_nsga3::{
    fast_nondominated_sort, hypervolume, igd_plus, normalize, polynomial_mutation, sbx_crossover,
    uniform_reference_points, Individual,
};

use support::json::{self, Json};
use support::rng::ScriptedUniform;

const FIXTURE: &str = include_str!("fixtures/conformance/component_oracle.json");
const TOL: f64 = 1e-9;

fn fixture() -> Json {
    json::parse_or_panic(FIXTURE)
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected}, got {actual} (|delta| = {})",
        (actual - expected).abs()
    );
}

fn candidate_from(genes: &[f64]) -> Candidate {
    Candidate::new(genes.iter().map(|&g| ValidFloat::new(g).unwrap()).collect()).unwrap()
}

fn variables_from_bounds(bounds: &[Vec<f64>]) -> Vec<Variable> {
    bounds
        .iter()
        .enumerate()
        .map(|(i, pair)| {
            Variable::new(
                VariableId(i as u64),
                VariableKind::continuous(
                    ValidFloat::new(pair[0]).unwrap(),
                    ValidFloat::new(pair[1]).unwrap(),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect()
}

fn usize_matrix(value: &Json) -> Vec<Vec<usize>> {
    value
        .as_array()
        .iter()
        .map(|row| row.vector().iter().map(|&v| v as usize).collect())
        .collect()
}

// ───────────────────────── reference lattices ─────────────────────────

#[test]
fn reference_lattices_match_pymoo() {
    let root = fixture();
    let cases = root.field("reference_lattices").as_array();
    assert!(!cases.is_empty(), "at least one lattice case is required");

    for case in cases {
        let name = case.field("name").text();
        let m = case.field("n_obj").usize();
        let p = case.field("divisions").usize();
        let expected = case.field("points").matrix();
        assert_eq!(case.field("count").usize(), expected.len(), "{name}: count");

        let actual = uniform_reference_points(m, p).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(actual.len(), expected.len(), "{name}: lattice size");
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert_eq!(a.len(), e.len(), "{name}: point {i} arity");
            for (j, (&av, &ev)) in a.iter().zip(e.iter()).enumerate() {
                assert_close(av, ev, 1e-15, &format!("{name}: point {i} component {j}"));
            }
        }
    }
}

// ───────────────────────── nondominated sort ─────────────────────────

#[test]
fn nondominated_sort_matches_deap() {
    let root = fixture();
    let case = root.field("nondominated_sort");
    let matrix = case.field("matrix").matrix();
    let expected_fronts = usize_matrix(case.field("fronts"));
    let expected_rank: Vec<usize> = case
        .field("rank")
        .vector()
        .iter()
        .map(|&v| v as usize)
        .collect();

    let actual_fronts = fast_nondominated_sort(&matrix).expect("validated matrix");
    assert_eq!(
        actual_fronts.len(),
        case.field("n_fronts").usize(),
        "front count"
    );

    // Rank-per-individual must match exactly.
    let mut actual_rank = vec![usize::MAX; matrix.len()];
    for (level, front) in actual_fronts.iter().enumerate() {
        for &idx in front {
            actual_rank[idx] = level;
        }
    }
    assert_eq!(actual_rank, expected_rank, "rank vector");

    // Each front's index set must match exactly (within-front order is not
    // semantically meaningful for a nondominated sort).
    for (level, (actual, expected)) in actual_fronts.iter().zip(expected_fronts.iter()).enumerate()
    {
        let mut a = actual.clone();
        let mut e = expected.clone();
        a.sort_unstable();
        e.sort_unstable();
        assert_eq!(a, e, "front {level} membership");
    }
}

// ───────────────────────── normalization ─────────────────────────

#[test]
fn normalize_matches_pymoo() {
    let root = fixture();
    let case = root.field("normalize");
    let matrix = case.field("matrix").matrix();
    let expected_ideal = case.field("ideal").vector();
    let expected_intercepts = case.field("intercepts").vector();
    let expected_normalized = case.field("normalized").matrix();

    let result = normalize(&matrix).expect("validated matrix");

    assert_eq!(result.ideal.len(), expected_ideal.len(), "ideal arity");
    for (j, (&a, &e)) in result.ideal.iter().zip(expected_ideal.iter()).enumerate() {
        assert_close(a, e, TOL, &format!("ideal[{j}]"));
    }
    assert_eq!(result.intercepts.len(), expected_intercepts.len());
    for (j, (&a, &e)) in result
        .intercepts
        .iter()
        .zip(expected_intercepts.iter())
        .enumerate()
    {
        assert_close(a, e, TOL, &format!("intercept[{j}]"));
    }
    assert_eq!(result.normalized.len(), expected_normalized.len());
    for (i, (row, expected)) in result
        .normalized
        .iter()
        .zip(expected_normalized.iter())
        .enumerate()
    {
        for (j, (&a, &e)) in row.iter().zip(expected.iter()).enumerate() {
            assert_close(a, e, TOL, &format!("normalized[{i}][{j}]"));
        }
    }
}

// ───────────────────────── hypervolume ─────────────────────────

#[test]
fn hypervolume_matches_deap_exact() {
    let root = fixture();
    let case = root.field("hypervolume");
    let front = case.field("front").matrix();
    let reference = case.field("reference_point").vector();
    let expected_deap = case.field("deap_hypervolume").number();
    let expected_independent = case.field("independent_hypervolume").number();

    let actual = hypervolume(&front, &reference).expect("validated front");
    assert_close(actual, expected_deap, TOL, "hypervolume vs DEAP exact");
    assert_close(
        actual,
        expected_independent,
        1e-9,
        "hypervolume vs independent in-script",
    );
}

// ───────────────────────── IGD+ ─────────────────────────

#[test]
fn igd_plus_matches_independent() {
    let root = fixture();
    let case = root.field("igd_plus");
    let front = case.field("front").matrix();
    let reference = case.field("reference_front").matrix();
    let expected = case.field("igd_plus").number();

    let actual = igd_plus(&front, &reference).expect("validated fronts");
    assert_close(actual, expected, TOL, "igd_plus vs independent in-script");
}

// ───────────────────────── genetic operators ─────────────────────────

#[test]
fn sbx_crossover_matches_deap_on_matched_stream() {
    let root = fixture();
    let case = root.field("sbx_crossover");
    let bounds = case.field("bounds").matrix();
    let parent1 = case.field("parent1").vector();
    let parent2 = case.field("parent2").vector();
    let eta = case.field("eta").number();
    let uniforms = case.field("ours_uniforms").vector();
    let expected_child1 = case.field("deap_child1").vector();
    let expected_child2 = case.field("deap_child2").vector();

    let vars = variables_from_bounds(&bounds);
    let mut c1 = Individual::new(candidate_from(&parent1));
    let mut c2 = Individual::new(candidate_from(&parent2));
    let mut rng = ScriptedUniform::new(uniforms);
    sbx_crossover(&mut c1, &mut c2, &vars, eta, &mut rng).expect("valid sbx inputs");

    let g1 = c1.genes();
    let g2 = c2.genes();
    for i in 0..bounds.len() {
        assert_close(g1[i], expected_child1[i], TOL, &format!("sbx child1[{i}]"));
        assert_close(g2[i], expected_child2[i], TOL, &format!("sbx child2[{i}]"));
    }
}

#[test]
fn polynomial_mutation_matches_deap_on_matched_stream() {
    let root = fixture();
    let case = root.field("polynomial_mutation");
    let bounds = case.field("bounds").matrix();
    let genes = case.field("genes").vector();
    let eta = case.field("eta").number();
    let uniforms = case.field("uniforms").vector();
    let expected = case.field("deap_mutated").vector();

    let vars = variables_from_bounds(&bounds);
    let mut individual = Individual::new(candidate_from(&genes));
    let mut rng = ScriptedUniform::new(uniforms);
    polynomial_mutation(&mut individual, &vars, eta, &mut rng).expect("valid mutation inputs");

    let actual = individual.genes();
    for i in 0..bounds.len() {
        assert_close(
            actual[i],
            expected[i],
            TOL,
            &format!("poly mutation gene[{i}]"),
        );
    }
}

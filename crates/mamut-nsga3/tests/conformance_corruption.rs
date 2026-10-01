//! Corruption-sensitivity conformance: the quality gate must FAIL for
//! deliberately corrupted NSGA-III components.
//!
//! Three mutants (each local to this test file, never in `src/`) are run through
//! the same test-local loop and compared against the strict band. The honest
//! baseline must PASS. Findings:
//!
//! * `flip_dominance` collapses DTLZ2 quality (dominance sign flipped).
//! * `always_first` degrades DTLZ2 partial-front selection.
//! * `bypass_normalize` is a documented no-op on DTLZ2 (DTLZ2 objectives are
//!   already near the unit simplex, so ideal translation + intercept scaling is
//!   close to identity) but measurably degrades DTLZ7, whose raw objectives span
//!   very different scales (`f3` in ~[2.6, 6] vs `f1, f2` in [0, 1]). The gate
//!   is therefore sensitive to normalization loss, exercised on the problem
//!   where normalization actually matters.
//!
//! This proves the conformance gate is sensitive (DoD: "corrupted NSGA
//! component must fail the conformance gate").

mod support;

use mamut_nsga3::{extract_pareto_front, hypervolume, igd_plus, is_nondominated_set};

use support::dtlz;
use support::harness::{
    baseline_pipeline, nondominated_rows, run_front, BandReport, BandSpec, Pipeline, SeedMetrics,
    SEEDS,
};
use support::json;

const FIXTURE: &str = include_str!("fixtures/conformance/pymoo_dtlz_oracle.json");

const STRICT: BandSpec = BandSpec {
    hv_min_ratio: 0.95,
    hv_degenerate_floor: 0.05,
    hv_abs_floor: 0.0,
    igd_floor: 0.0,
};

// ───────────────────────── corrupted components (test-local) ─────────────────

/// Maximization dominance (dominance sign flipped).
fn dominates_flipped(p: &[f64], q: &[f64]) -> bool {
    let mut strictly_better = false;
    for i in 0..p.len() {
        if p[i] < q[i] {
            return false;
        }
        if p[i] > q[i] {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Nondominated sort with flipped dominance (selects the worst trade-off set).
fn flipped_sort(fitnesses: &[Vec<f64>]) -> Vec<Vec<usize>> {
    let n = fitnesses.len();
    if n == 0 {
        return Vec::new();
    }
    let mut domination_count = vec![0usize; n];
    let mut dominated_set: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut front0 = Vec::new();
    for p in 0..n {
        for q in 0..n {
            if p == q {
                continue;
            }
            if dominates_flipped(&fitnesses[p], &fitnesses[q]) {
                dominated_set[p].push(q);
            } else if dominates_flipped(&fitnesses[q], &fitnesses[p]) {
                domination_count[p] += 1;
            }
        }
        if domination_count[p] == 0 {
            front0.push(p);
        }
    }
    let mut fronts = Vec::new();
    let mut current = front0;
    while !current.is_empty() {
        let mut next = Vec::new();
        for &p in &current {
            for &q in &dominated_set[p] {
                domination_count[q] -= 1;
                if domination_count[q] == 0 {
                    next.push(q);
                }
            }
        }
        fronts.push(current);
        current = next;
    }
    fronts
}

/// Normalization bypassed: return raw objectives unchanged (intercepts all 1).
fn bypass_normalize(fitnesses: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>) {
    let m = fitnesses.first().map_or(0, Vec::len);
    (fitnesses.to_vec(), vec![1.0; m])
}

/// Niche-count/partial-front selection replaced by "always take the first
/// remaining individual" (ignores niche counts and perpendicular distance).
fn always_first(
    partial_front: &[usize],
    _assoc: &[(usize, f64)],
    _niche_count: &mut [u32],
    needed: usize,
) -> Vec<usize> {
    partial_front.iter().copied().take(needed).collect()
}

fn mutant_pipeline(kind: &str) -> Pipeline {
    let base = baseline_pipeline();
    match kind {
        "flip_dominance" => Pipeline {
            sort: flipped_sort,
            normalize: base.normalize,
            partial_select: base.partial_select,
        },
        "bypass_normalize" => Pipeline {
            sort: base.sort,
            normalize: bypass_normalize,
            partial_select: base.partial_select,
        },
        "always_first" => Pipeline {
            sort: base.sort,
            normalize: base.normalize,
            partial_select: always_first,
        },
        other => panic!("unknown mutant {other}"),
    }
}

// ───────────────────────── harness ─────────────────────────

/// Run the strict band for `problem_name` with a given pipeline; return the
/// verdict and a rank-1 purity flag.
fn run_band(problem_name: &str, pipeline: &Pipeline) -> (support::harness::GateVerdict, bool) {
    let root = json::parse_or_panic(FIXTURE);
    let payload = root.field("problems").field(problem_name);
    let hv_reference = payload.field("hv_reference_point").vector();
    let reference_front = payload.field("pf_points").matrix();
    let runs = payload.field("runs").as_array();
    let oracle_hv: Vec<f64> = runs.iter().map(|r| r.field("hv").number()).collect();
    let oracle_igd: Vec<f64> = runs.iter().map(|r| r.field("igd_plus").number()).collect();
    let problem = dtlz::problem(problem_name);

    let mut ours = Vec::new();
    let mut pure = true;
    for &seed in &SEEDS {
        let fitnesses = run_front(&problem, seed, pipeline);
        let front_indices = extract_pareto_front(&fitnesses).unwrap();
        pure &= is_nondominated_set(&fitnesses, &front_indices).unwrap();
        let front = nondominated_rows(&fitnesses);
        ours.push(SeedMetrics {
            hv: hypervolume(&front, &hv_reference).expect("finite front"),
            igd_plus: igd_plus(&front, &reference_front).expect("finite front"),
        });
    }
    let report = BandReport {
        problem: problem.name,
        ours,
        oracle_hv,
        oracle_igd,
    };
    (support::harness::evaluate_gate(&report, STRICT), pure)
}

// ───────────────────────── tests ─────────────────────────

#[test]
fn honest_baseline_passes_dtlz2_strict_band() {
    let (verdict, pure) = run_band("dtlz2", &baseline_pipeline());
    println!("[corruption] baseline dtlz2 {}", verdict.detail);
    assert!(
        verdict.ok,
        "honest baseline must pass the strict DTLZ2 band"
    );
    assert!(pure, "baseline rank-1 front must be pure nondominated");
}

#[test]
fn corrupted_components_fail_the_gate() {
    // Dominance sign flip collapses DTLZ2.
    for kind in ["flip_dominance", "always_first"] {
        let (verdict, pure) = run_band("dtlz2", &mutant_pipeline(kind));
        println!(
            "[corruption] mutant={kind} problem=dtlz2 {}",
            verdict.detail
        );
        assert!(
            !verdict.ok || !pure,
            "mutant `{kind}` must fail the strict DTLZ2 gate or violate purity: {}",
            verdict.detail
        );
    }

    // Normalization bypass: documented no-op on DTLZ2, but a strong degradation
    // on DTLZ7 where objective scales differ.
    let (dtlz2_verdict, _) = run_band("dtlz2", &mutant_pipeline("bypass_normalize"));
    println!(
        "[corruption] mutant=bypass_normalize problem=dtlz2 (documented no-op) {}",
        dtlz2_verdict.detail
    );
    let (dtlz7_verdict, dtlz7_pure) = run_band("dtlz7", &mutant_pipeline("bypass_normalize"));
    println!(
        "[corruption] mutant=bypass_normalize problem=dtlz7 {}",
        dtlz7_verdict.detail
    );
    assert!(
        !dtlz7_verdict.ok || !dtlz7_pure,
        "normalization bypass must degrade DTLZ7: {}",
        dtlz7_verdict.detail
    );
}

//! MATH-VV-001 — Category C (fast slice): baseline distribution replay.
//!
//! Runs the honest DTLZ2 quality loop (the test-local, pymoo-faithful composition
//! in `support::harness`) for the documented fast subset `seeds 0..=1` of the
//! frozen 8-seed corpus and compares the observed per-seed HV / IGD+ to the
//! committed `tests/fixtures/regression/baseline_quality.json` at the same-build
//! tolerance `1e-12` (determinism, not approximation). It then checks the
//! committed stochastic band against the pymoo oracle medians.
//!
//! The committed thresholds are **reused verbatim from MATH-NSGA-004 and are not
//! retuned** here; the fixture stores them plus the per-problem rationale.

mod support;

use support::harness::{baseline_pipeline, evaluate_gate, seed_metrics, BandReport, SeedMetrics};
use support::regression::{
    applied_band_row, baseline, oracle_problem, FAST_SEEDS, SAME_BUILD_EXACT_TOLERANCE,
};

/// Category C fast slice — observed per-seed metrics equal the committed
/// baseline exactly (same-build determinism at `1e-12`).
#[test]
fn dtlz2_fast_slice_matches_committed_exactly() {
    let baseline = baseline();
    let payload = baseline.field("problems").field("dtlz2");
    let (hv_reference, reference_front, _, _) = oracle_problem("dtlz2");
    let problem = support::dtlz::problem("dtlz2");

    for &seed in &FAST_SEEDS {
        let observed = seed_metrics(
            &problem,
            seed,
            &hv_reference,
            &reference_front,
            &baseline_pipeline(),
        );
        let committed = &payload.field("per_seed").as_array()[seed as usize];
        let committed_hv = committed.field("hv").number();
        let committed_igd = committed.field("igd_plus").number();
        println!(
            "[vv001-C-fast] dtlz2 seed {seed}: hv={:.15e} igd_plus={:.15e}",
            observed.hv, observed.igd_plus
        );
        assert!(
            (observed.hv - committed_hv).abs() <= SAME_BUILD_EXACT_TOLERANCE,
            "dtlz2 seed {seed} HV drift: observed={} committed={} (tol {SAME_BUILD_EXACT_TOLERANCE})",
            observed.hv,
            committed_hv
        );
        assert!(
            (observed.igd_plus - committed_igd).abs() <= SAME_BUILD_EXACT_TOLERANCE,
            "dtlz2 seed {seed} IGD+ drift: observed={} committed={} (tol {SAME_BUILD_EXACT_TOLERANCE})",
            observed.igd_plus,
            committed_igd
        );
    }
}

/// Category C fast slice — the committed distribution satisfies the
/// pre-registered DTLZ2 strict band against the pymoo oracle medians, and the
/// live fast-slice replay is also consistent with the same band.
#[test]
fn dtlz2_fast_slice_band_passes() {
    let baseline = baseline();
    let payload = baseline.field("problems").field("dtlz2");
    let band = applied_band_row("dtlz2");
    let (_, _, oracle_hv, oracle_igd) = oracle_problem("dtlz2");

    // The authoritative check: the committed full-corpus distribution vs the
    // pre-registered band (thresholds reused, not retuned).
    let committed_ours: Vec<SeedMetrics> = payload
        .field("per_seed")
        .as_array()
        .iter()
        .map(|entry| SeedMetrics {
            hv: entry.field("hv").number(),
            igd_plus: entry.field("igd_plus").number(),
        })
        .collect();
    let committed = BandReport {
        problem: "dtlz2",
        ours: committed_ours.clone(),
        oracle_hv: oracle_hv.clone(),
        oracle_igd: oracle_igd.clone(),
    };
    let verdict = evaluate_gate(&committed, band.spec());
    println!("[vv001-C-fast] committed band {}", verdict.detail);
    assert!(
        payload.field("band_pass").bool(),
        "the committed baseline must be recorded as passing the band"
    );
    assert!(
        verdict.ok,
        "committed DTLZ2 band must pass: {}",
        verdict.detail
    );

    // The live fast slice is additionally checked against the same band.
    let (hv_reference, reference_front, _, _) = oracle_problem("dtlz2");
    let problem = support::dtlz::problem("dtlz2");
    let live_ours: Vec<SeedMetrics> = FAST_SEEDS
        .iter()
        .map(|&seed| {
            seed_metrics(
                &problem,
                seed,
                &hv_reference,
                &reference_front,
                &baseline_pipeline(),
            )
        })
        .collect();
    let live = BandReport {
        problem: "dtlz2",
        ours: live_ours,
        oracle_hv: FAST_SEEDS.iter().map(|&s| oracle_hv[s as usize]).collect(),
        oracle_igd: FAST_SEEDS.iter().map(|&s| oracle_igd[s as usize]).collect(),
    };
    let live_verdict = evaluate_gate(&live, band.spec());
    println!(
        "[vv001-C-fast] live fast-slice band {}",
        live_verdict.detail
    );
    assert!(
        live_verdict.ok,
        "live DTLZ2 fast-slice band must pass: {}",
        live_verdict.detail
    );
}

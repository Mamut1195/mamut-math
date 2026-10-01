//! MATH-VV-001 — SLOW lane: full frozen 8-seed corpus + certified reference.
//!
//! This test is **not part of the default `cargo test --workspace` run**: it is
//! annotated `#[ignore]`, so cargo compiles it but reports it as `ignored` and
//! does not execute it. Run it explicitly with:
//!
//! ```text
//! cargo test -p mamut-nsga3 --test regression_quality_slow -- --ignored
//! ```
//!
//! It runs the honest DTLZ1/2/7 quality loop for the full frozen corpus
//! (`seeds 0..=7`, pop 92, 9200 evaluations), checks each problem against its
//! pre-registered pymoo oracle band (reused from MATH-NSGA-004, not retuned),
//! checks the DTLZ2 strict band, re-executes the NSGA-004 certified DTLZ7
//! reference checks at full resolution, and writes `target/vv001_slow_report.json`
//! with the same `VV001RegressionReport/1` schema as the fast lane.

mod support;

use support::harness::{baseline_pipeline, evaluate_gate, seed_metrics, BandReport, SeedMetrics};
use support::regression::{
    applied_band_row, baseline, baseline_fixture_path, build_report, certified_dtlz7_checks,
    generate_baseline_quality_json, oracle_problem, strict_band_row, write_report, Lane,
    CORPUS_SEEDS, SAME_BUILD_EXACT_TOLERANCE,
};

#[test]
#[ignore = "MATH-VV-001 slow lane: full 8-seed corpus + certified reference; run with -- --ignored"]
fn full_corpus_bands_and_certified_reference() {
    let baseline_fixture = baseline();

    for name in ["dtlz1", "dtlz2", "dtlz7"] {
        let payload = baseline_fixture.field("problems").field(name);
        let (hv_reference, reference_front, oracle_hv, oracle_igd) = oracle_problem(name);
        let problem = support::dtlz::problem(name);

        let mut ours: Vec<SeedMetrics> = Vec::new();
        for (i, &seed) in CORPUS_SEEDS.iter().enumerate() {
            let observed = seed_metrics(
                &problem,
                seed,
                &hv_reference,
                &reference_front,
                &baseline_pipeline(),
            );
            let committed = &payload.field("per_seed").as_array()[i];
            assert!(
                (observed.hv - committed.field("hv").number()).abs() <= SAME_BUILD_EXACT_TOLERANCE,
                "{name} seed {seed} HV drift vs committed baseline"
            );
            assert!(
                (observed.igd_plus - committed.field("igd_plus").number()).abs()
                    <= SAME_BUILD_EXACT_TOLERANCE,
                "{name} seed {seed} IGD+ drift vs committed baseline"
            );
            ours.push(observed);
        }

        let report = BandReport {
            problem: problem.name,
            ours,
            oracle_hv,
            oracle_igd,
        };
        let applied = evaluate_gate(&report, applied_band_row(name).spec());
        println!("[vv001-slow] {name} applied {}", applied.detail);
        assert!(
            applied.ok,
            "{name} failed its applied band: {}",
            applied.detail
        );

        if name == "dtlz2" {
            let strict = evaluate_gate(&report, strict_band_row().spec());
            println!("[vv001-slow] {name} strict {}", strict.detail);
            assert!(
                strict.ok,
                "DTLZ2 must pass the strict band: {}",
                strict.detail
            );
        }
    }

    let (cert_ok, cert_detail) = certified_dtlz7_checks();
    println!("[vv001-slow] certified-reference {cert_detail}");
    assert!(
        cert_ok,
        "certified DTLZ7 reference checks failed: {cert_detail}"
    );

    let report = build_report(Lane::Slow);
    write_report("vv001_slow_report.json", &report);
    println!("[vv001-slow] wrote target/vv001_slow_report.json");
}

/// Offline fixture regeneration entry point (not a lane): recomputes the
/// committed baseline-quality fixture from the deterministic pipeline and writes
/// it over `tests/fixtures/regression/baseline_quality.json`. Manual use only
/// (`-- --ignored --exact`), after reviewing the resulting diff; it proves the
/// committed fixture - sensitivity ledger included - is recomputable
/// byte-for-byte from the same source SHA.
#[test]
#[ignore = "MATH-VV-001 offline regeneration: rewrites a committed fixture"]
fn regenerate_baseline_quality_fixture() {
    let text = generate_baseline_quality_json();
    let path = baseline_fixture_path();
    std::fs::write(&path, text).expect("fixture writable");
    println!("[vv001-regen] rewrote {}", path.display());
}

//! MATH-VV-001 — Category E: corruption / sensitivity.
//!
//! This file **reuses** the existing `conformance_corruption.rs` suite (which
//! proves the quality gate fails for deliberately corrupted NSGA-III
//! components); it does not duplicate those mutants. Here we assert that the
//! VV-001 machine-readable report carries the corruption lane as explicit
//! gate-sensitivity evidence, and that the one-off local mutations performed
//! while implementing the ticket (a flipped expected hash and a flipped
//! committed per-seed HV value) each failed their gate before being restored
//! byte-exact.

mod support;

use support::regression::{baseline, build_report, repo_root, Category, Lane, Status};

/// The referenced corruption suite exists and still carries its three mutants.
#[test]
fn corruption_suite_is_referenced_not_duplicated() {
    let path = repo_root().join("crates/mamut-nsga3/tests/conformance_corruption.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} unreadable: {error}", path.display()));
    for mutant in ["flip_dominance", "always_first", "bypass_normalize"] {
        assert!(
            source.contains(mutant),
            "referenced corruption suite must still exercise mutant `{mutant}`"
        );
    }
}

/// The fast report exposes category E as explicit gate-sensitivity evidence.
#[test]
fn fast_report_marks_corruption_as_sensitivity_evidence() {
    let report = build_report(Lane::Fast);
    let entry = report
        .gates
        .iter()
        .find(|gate| gate.category == Category::CorruptionSensitivity)
        .expect("category E gate must exist in the report");
    assert_eq!(entry.status, Status::Pass);
    assert!(
        entry.observed.contains("conformance_corruption"),
        "E gate must reference the corruption suite, got: {}",
        entry.observed
    );
    assert!(!entry.rationale.is_empty());
}

/// The committed sensitivity ledger records that the one-off mutations failed
/// and were restored (non-vacuous sensitivity).
#[test]
fn committed_sensitivity_ledger_is_non_vacuous() {
    let baseline = baseline();
    let ledger = baseline
        .field("sensitivity_evidence")
        .field("ledger")
        .as_array();
    assert!(
        ledger.len() >= 2,
        "the ledger must record at least the hash-flip and baseline-HV-flip mutations"
    );
    for entry in ledger {
        assert!(
            entry.field("expected_failure").bool(),
            "a sensitivity probe must expect failure"
        );
        assert!(
            entry.field("observed_failure").bool(),
            "the mutated gate must have actually failed"
        );
        assert!(
            entry.field("restored").bool(),
            "the mutated file must have been restored (never committed)"
        );
        assert!(!entry.field("failing_test").text().is_empty());
    }
}

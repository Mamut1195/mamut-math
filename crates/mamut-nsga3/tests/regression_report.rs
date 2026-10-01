//! MATH-VV-001 — Category D: the machine-readable regression report.
//!
//! Validates the `VV001RegressionReport/1` schema produced by
//! `support::regression::build_report` / `report_to_json`, writes the fast-lane
//! report to `target/vv001_fast_report.json`, and checks:
//!
//! * every committed regression fixture's recomputed SHA-256 equals the
//!   committed registry entry (using [`mamut_math_core::ArtifactHash::sha256`]);
//! * every category **A..E** is present and structurally explicit;
//! * every gate has a `PASS`/`FAIL` status and non-empty thresholds, observed
//!   values and rationale (reasons).
//!
//! ## MATH-EVD-001 alignment
//!
//! The report reuses the core evidence contracts mechanically:
//! [`mamut_math_core::ArtifactHash`] for the fixture digests,
//! [`mamut_math_core::RngSpec`] for the RNG identity and
//! [`mamut_math_core::SolverVersion`] for the algorithm identity. These are the
//! same types embedded by [`mamut_math_core::RunManifest`], the canonical
//! evidence record (`Fingerprint`, `RunKind`, `ArtifactHash`, `SolverVersion`,
//! `RngSpec`); the report is a test-side projection of that contract and does
//! not introduce a parallel evidence vocabulary.

mod support;

use mamut_math_core::{ArtifactHash, RngSpec, SolverVersion, RNG_ALGORITHM_ID};
use support::json;
use support::regression::{
    build_report, file_sha256_hex, fixture_registry, report_to_json, write_report, Category, Lane,
    REGISTRY_FILES,
};

/// Category D: the fast report is written, well-formed and complete.
#[test]
fn fast_report_validates_schema_categories_and_registry() {
    let report = build_report(Lane::Fast);
    write_report("vv001_fast_report.json", &report);

    let json_text = report_to_json(&report);
    let parsed = json::parse_or_panic(&json_text);
    assert_eq!(
        parsed.field("schema_version").text(),
        "VV001RegressionReport/1"
    );
    assert_eq!(parsed.field("report_id").text(), "vv001-fast");
    assert!(
        !parsed.field("code_sha").text().is_empty(),
        "code_sha must be non-empty"
    );
    assert_eq!(parsed.field("algorithm").field("id").text(), "mamut-nsga3");
    assert_eq!(
        parsed.field("rng").field("id").text(),
        RNG_ALGORITHM_ID,
        "report must record the frozen RNG algorithm id"
    );

    // Every category A..E must be explicit in the report.
    let present: Vec<String> = parsed
        .field("gates")
        .as_array()
        .iter()
        .map(|entry| entry.field("category").text().to_string())
        .collect();
    for category in Category::ALL {
        assert!(
            present.iter().any(|tag| tag == category.tag()),
            "category {} ({}) missing from the fast report",
            category.tag(),
            category.title()
        );
    }

    // Every gate must carry a PASS/FAIL status and non-empty reasons.
    for entry in parsed.field("gates").as_array() {
        let status = entry.field("status").text();
        assert!(
            status == "PASS" || status == "FAIL",
            "gate {} has invalid status {status}",
            entry.field("id").text()
        );
        assert!(
            !entry.field("rationale").text().is_empty(),
            "gate {} must carry a non-empty rationale",
            entry.field("id").text()
        );
        assert!(!entry.field("thresholds").text().is_empty());
        assert!(!entry.field("observed").text().is_empty());
    }
}

/// Category D: each committed fixture hashes to its committed registry digest.
#[test]
fn fixture_registry_hashes_are_current() {
    let registry = fixture_registry();
    assert!(
        registry.len() >= REGISTRY_FILES.len(),
        "registry must cover at least the declared regression fixtures"
    );
    let registered_paths: Vec<&str> = registry.iter().map(|(path, _)| path.as_str()).collect();
    for required in REGISTRY_FILES {
        assert!(
            registered_paths.contains(&required),
            "fixture registry must include {required}"
        );
    }
    for (path, committed) in &registry {
        let recomputed = file_sha256_hex(path);
        assert_eq!(
            &recomputed, committed,
            "fixture {path} drifted: recomputed={recomputed} committed={committed}"
        );
    }
}

/// Category D: prove the report reuses the MATH-EVD-001 evidence types (not a
/// parallel vocabulary).
#[test]
fn report_reuses_core_evidence_types() {
    // ArtifactHash is the same digest machinery used by RunManifest::artifacts.
    let artifact = ArtifactHash::sha256("vv001-registry".to_string(), b"mamut").unwrap();
    assert_eq!(artifact.algorithm(), "sha256-v1");
    assert_eq!(artifact.digest().len(), 32);

    // RngSpec is the plain-data RNG identity embedded by RunKind::Stochastic.
    let rng = RngSpec::new(RNG_ALGORITHM_ID.to_string(), "1".to_string()).unwrap();
    assert_eq!(rng.algorithm(), RNG_ALGORITHM_ID);

    // SolverVersion is the algorithm identity embedded by RunManifest.
    let algorithm = SolverVersion::new("mamut-nsga3".to_string(), "0.1.0".to_string()).unwrap();
    assert_eq!(algorithm.id(), "mamut-nsga3");

    // The live report carries the same identities.
    let report = build_report(Lane::Fast);
    assert_eq!(report.rng.algorithm(), RNG_ALGORITHM_ID);
    assert_eq!(report.algorithm.id(), "mamut-nsga3");
}

//! MATH-VV-001 — Category A: legacy parity (same-build seed replay + provenance).
//!
//! ## Lane A reuse note
//!
//! This file covers the *new* same-build seed-replay and provenance anchors.
//! The pre-existing legacy component-parity suites are part of the same lane
//! and are **referenced, not duplicated** here:
//!
//! * `ported_golden.rs` — frozen Hydraulics `nsga3/*` expected values;
//! * `scripted_parity.rs` — scripted-RNG differential operator parity;
//! * `session.rs` — ask/tell session structure and admission semantics.
//!
//! The same-build replay drives the frozen core [`mamut_math_core::rng::MamutRng`]
//! through a fixed short component pipeline and pins the SHA-256 of a canonical
//! byte trace. Hashing uses the MAMUT-owned evidence machinery
//! ([`mamut_math_core::ArtifactHash::sha256`]); there is no ad-hoc hash here.
//!
//! Legacy parity means **we preserve what we decided to preserve, NOT
//! mathematical authority** (see `docs/validation/VV001-REGRESSION.md`).

mod support;

use support::regression::{
    legacy_provenance, legacy_seed_replay, legacy_trace, repo_root, rng_spec, validation_doc_path,
    LEGACY_PARITY_STATEMENT, LEGACY_SEEDS, LEGACY_SOURCE_SHA,
};

/// Category A(a): the same-build seed replay is bit-exact for the whole
/// committed seed set.
#[test]
fn seed_replay_matches_committed_hashes() {
    let replay = legacy_seed_replay();
    for (i, (seed, actual, expected, ok)) in replay.iter().enumerate() {
        println!("[vv001-A] seed {seed}: {actual}");
        assert!(
            ok,
            "legacy trace hash mismatch for seed {seed}: actual={actual} \
             expected={expected} (LEGACY_EXPECTED_HASHES[{i}])"
        );
    }
    assert_eq!(replay.len(), LEGACY_SEEDS.len());
}

/// Category A(a): replaying twice in the same build gives the same digest, and
/// distinct seeds give distinct digests (the anchor is not vacuously constant).
#[test]
fn seed_replay_is_repeatable_and_seed_sensitive() {
    let (bytes_a, _) = legacy_trace(LEGACY_SEEDS[0]);
    let (bytes_b, _) = legacy_trace(LEGACY_SEEDS[0]);
    assert_eq!(bytes_a, bytes_b, "same seed must replay bit-exactly");

    let hash_a = support::regression::legacy_trace_hash(LEGACY_SEEDS[0]);
    let hash_b = support::regression::legacy_trace_hash(LEGACY_SEEDS[1]);
    assert_ne!(
        hash_a, hash_b,
        "distinct seeds must produce distinct traces"
    );
}

/// Category A(a): the RNG metadata recorded with the trace matches the frozen
/// core contract and the `RngSpec` used by the evidence report.
#[test]
fn rng_metadata_matches_frozen_contract() {
    let (_, metadata) = legacy_trace(0);
    let spec = rng_spec();
    assert_eq!(metadata.algorithm_id(), spec.algorithm());
    assert_eq!(
        metadata.algorithm_version().to_string(),
        spec.version(),
        "recorded RNG algorithm version must match the report RngSpec"
    );
    assert_eq!(
        metadata.algorithm_id(),
        mamut_math_core::RNG_ALGORITHM_ID,
        "the frozen algorithm id is mamut.chacha20-std"
    );
    assert!(
        metadata.canonical_bytes().is_ok(),
        "metadata must have a canonical v1 encoding"
    );
}

/// Category A(b): the committed provenance fixture states the frozen source SHA
/// and labels the semantics.
#[test]
fn legacy_provenance_fixture_labels_semantics() {
    let fixture = legacy_provenance();
    assert_eq!(fixture.field("source_sha").text(), LEGACY_SOURCE_SHA);
    assert_eq!(fixture.field("semantic_label").text(), "legacy-parity");
    assert_eq!(
        fixture.field("legacy_parity_statement").text(),
        LEGACY_PARITY_STATEMENT
    );
    let suites: Vec<&str> = fixture
        .field("referenced_suites")
        .as_array()
        .iter()
        .map(|entry| entry.text())
        .collect();
    for required in ["ported_golden.rs", "scripted_parity.rs", "session.rs"] {
        assert!(
            suites.contains(&required),
            "legacy provenance must reference the lane-A suite {required}"
        );
    }
}

/// Category A(b): the pre-existing committed fixtures / ported suite state the
/// same frozen source SHA, and the lane-A suite files exist.
#[test]
fn legacy_sha_is_recorded_in_committed_artifacts() {
    let root = repo_root();
    let metrics_fixture = std::fs::read_to_string(
        root.join("crates/mamut-nsga3/tests/fixtures/metrics_5d_golden.json"),
    )
    .expect("metrics_5d_golden.json must be committed");
    assert!(
        metrics_fixture.contains(LEGACY_SOURCE_SHA),
        "metrics_5d_golden.json must state the frozen source SHA"
    );

    let ported = std::fs::read_to_string(root.join("crates/mamut-nsga3/tests/ported_golden.rs"))
        .expect("ported_golden.rs must exist");
    assert!(
        ported.contains(LEGACY_SOURCE_SHA),
        "ported_golden.rs must state the frozen source SHA"
    );

    for suite in ["ported_golden.rs", "scripted_parity.rs", "session.rs"] {
        let path = root.join("crates/mamut-nsga3/tests").join(suite);
        assert!(path.exists(), "lane-A referenced suite {suite} must exist");
    }
}

/// Category A(b): the validation document carries the verbatim legacy-parity
/// semantics and an explicit A..E category mapping (DoD wording present).
#[test]
fn validation_doc_states_legacy_semantics_and_categories() {
    let path = validation_doc_path();
    let doc = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} unreadable: {error}", path.display()));
    assert!(
        doc.contains(LEGACY_PARITY_STATEMENT),
        "the doc must state the legacy-parity semantics verbatim"
    );
    for category in ["A", "B", "C", "D", "E"] {
        assert!(
            doc.contains(&format!("Category {category}")),
            "the doc must label Category {category}"
        );
    }
}

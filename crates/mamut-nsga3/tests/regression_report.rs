//! MATH-VV-001 — Category D: the machine-readable regression report.
//!
//! Validates the `VV001RegressionReport/1` schema produced by
//! `support::regression::build_report` / `report_to_json`, writes the fast-lane
//! report to `target/vv001_fast_report.json`, and checks:
//!
//! * each fixture matches its committed registry entry under its named policy;
//! * report `sha256-v1` digests describe actual raw on-disk bytes (using
//!   [`mamut_math_core::ArtifactHash::sha256`]);
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
    build_report, file_sha256_hex, fixture_registry, hex, registry_sha256_hex, repo_root,
    report_to_json, sha256_hex, write_report, Category, Lane, CANONICAL_CRLF_V1_FILES,
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

    // Read actual bytes independently of the registry comparison policy. This
    // must also hold on an LF checkout, where five committed digests differ.
    let artifacts = parsed.field("artifact_hashes").as_array();
    assert_eq!(artifacts.len(), REGISTRY_FILES.len());
    assert_eq!(report.fixture_registry.len(), REGISTRY_FILES.len());
    for path in REGISTRY_FILES {
        let bytes = std::fs::read(repo_root().join(path)).unwrap();
        let raw = ArtifactHash::sha256(path.to_string(), &bytes).unwrap();
        let expected = hex(raw.digest());
        assert_eq!(file_sha256_hex(path), expected);
        let entries: Vec<_> = artifacts
            .iter()
            .filter(|entry| entry.field("name").text() == path)
            .collect();
        assert_eq!(entries.len(), 1, "exactly one report artifact for {path}");
        assert_eq!(entries[0].field("algorithm").text(), "sha256-v1");
        assert_eq!(entries[0].field("digest").text(), expected);
        assert_eq!(
            report.fixture_registry.iter().find(|(p, _)| p == path),
            Some(&(path.to_string(), expected))
        );
    }
}

/// Category D: exact registry coverage and each fixture's comparison digest.
#[test]
fn fixture_registry_hashes_are_current() {
    let registry = fixture_registry();
    let mut registered_paths: Vec<&str> = registry.iter().map(|(path, _)| path.as_str()).collect();
    registered_paths.sort_unstable();
    let mut required_paths = REGISTRY_FILES.to_vec();
    required_paths.sort_unstable();
    assert_eq!(registered_paths, required_paths, "exact registry coverage");
    for (path, committed) in &registry {
        let bytes = std::fs::read(repo_root().join(path)).unwrap();
        let recomputed = registry_sha256_hex(path, &bytes)
            .unwrap_or_else(|error| panic!("fixture {path}: {error}"));
        assert_eq!(
            &recomputed, committed,
            "fixture {path} drifted: recomputed={recomputed} committed={committed}"
        );
    }
}

#[test]
fn registry_policy_canonical_crlf_v1_accepts_only_eol_equivalence() {
    let lf = b"\0\xff text\nlast\t \n";
    let crlf = b"\0\xff text\r\nlast\t \r\n";
    let mixed = b"\0\xff text\nlast\t \r\n";
    for path in CANONICAL_CRLF_V1_FILES {
        let expected = sha256_hex(path, crlf);
        for bytes in [lf.as_slice(), crlf.as_slice(), mixed.as_slice()] {
            assert_eq!(registry_sha256_hex(path, bytes).unwrap(), expected);
        }
        for (i, byte) in lf.iter().enumerate() {
            if *byte != b'\n' {
                let mut changed = lf.to_vec();
                changed[i] ^= 1;
                assert_ne!(registry_sha256_hex(path, &changed).unwrap(), expected);
            }
        }
        // A final newline, whitespace and BOM are content, not normalization.
        for changed in [
            b"\0\xff text\nlast\t ".as_slice(),
            b"\0\xff text\nlast\t\n".as_slice(),
            b"\xef\xbb\xbf\0\xff text\nlast\t \n".as_slice(),
        ] {
            assert_ne!(registry_sha256_hex(path, changed).unwrap(), expected);
        }
        for byte in 0..=u8::MAX {
            if byte != b'\r' && byte != b'\n' {
                assert_eq!(
                    registry_sha256_hex(path, &[byte]).unwrap(),
                    sha256_hex(path, &[byte]),
                    "non-EOL byte {byte} must be preserved for {path}"
                );
            }
        }
        assert_eq!(
            registry_sha256_hex(path, b"").unwrap(),
            sha256_hex(path, b"")
        );
    }
}

#[test]
fn registry_policy_canonical_crlf_v1_rejects_lone_cr() {
    for path in CANONICAL_CRLF_V1_FILES {
        for bytes in [b"\r".as_slice(), b"\ra", b"\r\nx\r", b"\r\r\n"] {
            assert_eq!(
                registry_sha256_hex(path, bytes),
                Err("canonical-crlf-v1 rejects lone CR")
            );
        }
    }
}

#[test]
fn registry_policy_sixth_and_future_paths_keep_raw_bytes() {
    for path in [
        "crates/mamut-nsga3/tests/fixtures/regression/legacy_provenance.json",
        "crates/mamut-nsga3/tests/fixtures/conformance/future_oracle.json",
        "component_oracle.json",
    ] {
        for bytes in [b"raw\n".as_slice(), b"raw\r\n", b"raw\r", b"\0\xff"] {
            assert_eq!(
                registry_sha256_hex(path, bytes).unwrap(),
                sha256_hex(path, bytes)
            );
        }
        assert_ne!(
            registry_sha256_hex(path, b"raw\n").unwrap(),
            registry_sha256_hex(path, b"raw\r\n").unwrap()
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

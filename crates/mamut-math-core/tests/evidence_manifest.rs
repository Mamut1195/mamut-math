//! MATH-EVD-001: run-manifest evidence contracts — round-trips, golden byte
//! fixtures, schema-negative tests, canonical JSON and a backend-neutral
//! Hydraulics example (metadata strings only; no domain types or filesystem).

use mamut_math_core::{
    decode_constraint_descriptors, decode_objective_descriptors, decode_run_manifest,
    encode_constraint_descriptors, encode_objective_descriptors, encode_run_manifest, ArtifactHash,
    ConfigParam, ConfigValue, ConstraintDescriptor, ConstraintKind, Diagnostic, DiagnosticCode,
    Diagnostics, ErrorKind, EvaluationRecord, EvidenceExport, Fingerprint, ObjectiveDescriptor,
    ObjectiveSense, Precision, RunId, RunKind, RunManifest, RunManifestParams, SolverVersion,
    StopReason, ValidFloat, Variable, VariableId, VariableKind, FINGERPRINT_ALGORITHM_SHA256_V1,
    MANIFEST_VERSION,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn vf(value: f64) -> ValidFloat {
    ValidFloat::new(value).expect("finite literal")
}

fn solver(id: &str, version: &str) -> SolverVersion {
    SolverVersion::new(id.to_string(), version.to_string()).unwrap()
}

fn diag(code: DiagnosticCode, note: &str) -> Diagnostic {
    Diagnostic {
        code,
        note: note.to_string(),
    }
}

/// A minimal valid manifest with the given run kind and nothing else.
fn minimal_params(run_kind: RunKind) -> RunManifestParams {
    RunManifestParams {
        manifest_version: MANIFEST_VERSION,
        run_id: RunId::new(7),
        fingerprint: Fingerprint::new(FINGERPRINT_ALGORITHM_SHA256_V1.to_string(), [0u8; 32])
            .unwrap(),
        solver_versions: vec![solver("s", "1")],
        run_kind,
        params: Vec::new(),
        precision: Precision::new(vf(1e-9), vf(1e-6)).unwrap(),
        target: None,
        evaluation: EvaluationRecord::new(0, 0, 0, None),
        stop_reason: StopReason::Converged,
        diagnostics: Diagnostics::new(Vec::new()),
        artifacts: Vec::new(),
    }
}

fn minimal_manifest() -> RunManifest {
    RunManifest::new(minimal_params(RunKind::deterministic())).unwrap()
}

fn stochastic_minimal_manifest() -> RunManifest {
    RunManifest::new(minimal_params(
        RunKind::stochastic(0, "xoshiro256pp-v1".to_string(), "1".to_string()).unwrap(),
    ))
    .unwrap()
}

/// A carrier manifest with one parameter, one target and one solver; used for
/// tag/offset tests. Layout (bytes): run_kind @83, config tag @96, logical @97,
/// target tag @114, stop @152 (carrier length 161).
fn carrier_params() -> RunManifestParams {
    let mut params = minimal_params(RunKind::deterministic());
    params.params = vec![ConfigParam::new("flag".to_string(), ConfigValue::Logical(true)).unwrap()];
    params.target = Some(vec![vf(1.0)]);
    params
}

fn carrier_manifest_with_stop(stop_reason: StopReason) -> RunManifest {
    let mut params = carrier_params();
    params.stop_reason = stop_reason;
    RunManifest::new(params).unwrap()
}

fn stop_variants() -> [StopReason; 8] {
    [
        StopReason::Converged,
        StopReason::TargetAchieved,
        StopReason::EvaluationBudgetExhausted,
        StopReason::IterationBudgetExhausted,
        StopReason::LogicalDeadlineReached { ticks: 12345 },
        StopReason::Aborted,
        StopReason::Failed,
        StopReason::Unknown,
    ]
}

fn problem() -> (
    Vec<Variable>,
    Vec<ObjectiveDescriptor>,
    Vec<ConstraintDescriptor>,
) {
    let variables = vec![
        Variable::new(
            VariableId(0),
            VariableKind::continuous(vf(0.0), vf(10.0)).unwrap(),
        )
        .unwrap(),
        Variable::new(VariableId(1), VariableKind::integer(-5, 5).unwrap()).unwrap(),
        Variable::new(VariableId(2), VariableKind::categorical(3).unwrap()).unwrap(),
    ];
    let objectives = vec![
        ObjectiveDescriptor::new(0, ObjectiveSense::Minimize),
        ObjectiveDescriptor::new(1, ObjectiveSense::Maximize),
    ];
    let constraints = vec![
        ConstraintDescriptor::new(0, ConstraintKind::Hard).unwrap(),
        ConstraintDescriptor::new(1, ConstraintKind::soft(vf(0.1)).unwrap()).unwrap(),
    ];
    (variables, objectives, constraints)
}

fn hex32(digest: &[u8; 32]) -> String {
    use core::fmt::Write as _;
    let mut out = String::new();
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// ---------------------------------------------------------------------------
// Golden fixtures. Each array is the exact expected canonical byte stream;
// comments annotate every field in order (SCHEMA_VERSION = 1).
// ---------------------------------------------------------------------------

/// Full, every-field-populated stochastic manifest.
fn full_manifest() -> RunManifest {
    let mut digest = [0u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = index as u8;
    }
    let run_id = RunId::new(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10);
    let fingerprint = Fingerprint::new(FINGERPRINT_ALGORITHM_SHA256_V1.to_string(), digest);
    let solver_versions = vec![solver("solver-a", "1.0.0"), solver("solver-b", "2.1.3")];
    let run_kind = RunKind::stochastic(
        0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10,
        "xoshiro256pp-v1".to_string(),
        "1".to_string(),
    )
    .unwrap();
    let params = vec![
        ConfigParam::new("alpha".to_string(), ConfigValue::Float(vf(2.5))).unwrap(),
        ConfigParam::new("beta".to_string(), ConfigValue::Integer(-3)).unwrap(),
        ConfigParam::new("gamma".to_string(), ConfigValue::Text("hi".to_string())).unwrap(),
        ConfigParam::new("delta".to_string(), ConfigValue::Logical(true)).unwrap(),
    ];
    let artifacts = vec![
        ArtifactHash::new("report".to_string(), "sha256-v1".to_string(), [0x11; 32]).unwrap(),
        ArtifactHash::new("trace".to_string(), "sha256-v1".to_string(), [0x22; 32]).unwrap(),
    ];
    RunManifest::new(RunManifestParams {
        manifest_version: MANIFEST_VERSION,
        run_id,
        fingerprint: fingerprint.unwrap(),
        solver_versions,
        run_kind,
        params,
        precision: Precision::new(vf(1e-9), vf(1e-6)).unwrap(),
        target: Some(vec![vf(1.0), vf(2.0)]),
        evaluation: EvaluationRecord::new(100, 2, 100, Some(200)),
        stop_reason: StopReason::LogicalDeadlineReached { ticks: 300 },
        diagnostics: Diagnostics::new(vec![diag(DiagnosticCode::EvaluationWarning, "ok")]),
        artifacts,
    })
    .unwrap()
}

const FULL_GOLDEN: [u8; 425] = [
    // SCHEMA_VERSION = 1
    0x01, 0x00, 0x00, 0x00, // manifest_version = 1
    0x01, 0x00, 0x00, 0x00, // run_id = 0x0102030405060708090A0B0C0D0E0F10 (u128 LE)
    0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02,
    0x01, // fingerprint algorithm "sha256-v1" (byte length 9)
    0x09, 0x00, 0x00, 0x00, 0x73, 0x68, 0x61, 0x32, 0x35, 0x36, 0x2D, 0x76, 0x31,
    // fingerprint digest = bytes 00..1F
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
    0x1F, // solver_versions count = 2
    0x02, 0x00, 0x00, 0x00, // solver-a id "solver-a" (byte length 8)
    0x08, 0x00, 0x00, 0x00, 0x73, 0x6F, 0x6C, 0x76, 0x65, 0x72, 0x2D, 0x61,
    // solver-a version "1.0.0" (byte length 5)
    0x05, 0x00, 0x00, 0x00, 0x31, 0x2E, 0x30, 0x2E, 0x30,
    // solver-b id "solver-b" (byte length 8)
    0x08, 0x00, 0x00, 0x00, 0x73, 0x6F, 0x6C, 0x76, 0x65, 0x72, 0x2D, 0x62,
    // solver-b version "2.1.3" (byte length 5)
    0x05, 0x00, 0x00, 0x00, 0x32, 0x2E, 0x31, 0x2E, 0x33,
    // run_kind tag = 1 (Stochastic)
    0x01, // seed = 0x0102030405060708090A0B0C0D0E0F10 (u128 LE)
    0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02,
    0x01, // rng algorithm "xoshiro256pp-v1" (byte length 15)
    0x0F, 0x00, 0x00, 0x00, 0x78, 0x6F, 0x73, 0x68, 0x69, 0x72, 0x6F, 0x32, 0x35, 0x36, 0x70, 0x70,
    0x2D, 0x76, 0x31, // rng version "1" (byte length 1)
    0x01, 0x00, 0x00, 0x00, 0x31, // params count = 4
    0x04, 0x00, 0x00, 0x00, // alpha: name (len 5) + Float tag 0 + 2.5 bits
    0x05, 0x00, 0x00, 0x00, 0x61, 0x6C, 0x70, 0x68, 0x61, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x04, 0x40, // beta: name (len 4) + Integer tag 1 + -3 (i64 LE)
    0x04, 0x00, 0x00, 0x00, 0x62, 0x65, 0x74, 0x61, 0x01, 0xFD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, // gamma: name (len 5) + Text tag 2 + "hi" (len 2)
    0x05, 0x00, 0x00, 0x00, 0x67, 0x61, 0x6D, 0x6D, 0x61, 0x02, 0x02, 0x00, 0x00, 0x00, 0x68, 0x69,
    // delta: name (len 5) + Logical tag 3 + true (1)
    0x05, 0x00, 0x00, 0x00, 0x64, 0x65, 0x6C, 0x74, 0x61, 0x03, 0x01,
    // precision absolute 1e-9 (bits LE), relative 1e-6 (bits LE)
    0x95, 0xD6, 0x26, 0xE8, 0x0B, 0x2E, 0x11, 0x3E, 0x8D, 0xED, 0xB5, 0xA0, 0xF7, 0xC6, 0xB0, 0x3E,
    // target tag = 1 (Some), count = 2, then 1.0 and 2.0
    0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x40,
    // evaluation: completed 100, failed 2, ticks 100 (u64 LE each)
    0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // budget tag = 1 (Some), budget = 200 (u64 LE)
    0x01, 0xC8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // stop_reason tag = 4 (LogicalDeadlineReached), ticks = 300 (u64 LE)
    0x04, 0x2C, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // diagnostics count = 1, code = 1 (EvaluationWarning), note "ok" (len 2)
    0x01, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x6F, 0x6B,
    // artifacts count = 2
    0x02, 0x00, 0x00, 0x00,
    // artifact "report": name (len 6), algorithm "sha256-v1", digest 0x11 * 32
    0x06, 0x00, 0x00, 0x00, 0x72, 0x65, 0x70, 0x6F, 0x72, 0x74, 0x09, 0x00, 0x00, 0x00, 0x73, 0x68,
    0x61, 0x32, 0x35, 0x36, 0x2D, 0x76, 0x31, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    // artifact "trace": name (len 5), algorithm "sha256-v1", digest 0x22 * 32
    0x05, 0x00, 0x00, 0x00, 0x74, 0x72, 0x61, 0x63, 0x65, 0x09, 0x00, 0x00, 0x00, 0x73, 0x68, 0x61,
    0x32, 0x35, 0x36, 0x2D, 0x76, 0x31, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
];

const MINIMAL_GOLDEN: [u8; 139] = [
    // SCHEMA_VERSION = 1
    0x01, 0x00, 0x00, 0x00, // manifest_version = 1
    0x01, 0x00, 0x00, 0x00, // run_id = 7 (u128 LE)
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // fingerprint algorithm "sha256-v1" (byte length 9)
    0x09, 0x00, 0x00, 0x00, 0x73, 0x68, 0x61, 0x32, 0x35, 0x36, 0x2D, 0x76, 0x31,
    // fingerprint digest = 32 zero bytes
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // solver_versions count = 1, id "s" (len 1), version "1" (len 1)
    0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x73, 0x01, 0x00, 0x00, 0x00, 0x31,
    // run_kind tag = 0 (Deterministic)
    0x00, // params count = 0
    0x00, 0x00, 0x00, 0x00, // precision absolute 1e-9, relative 1e-6 (f64 bits LE)
    0x95, 0xD6, 0x26, 0xE8, 0x0B, 0x2E, 0x11, 0x3E, 0x8D, 0xED, 0xB5, 0xA0, 0xF7, 0xC6, 0xB0, 0x3E,
    // target tag = 0 (None)
    0x00, // evaluation completed 0, failed 0, ticks 0 (u64 LE)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // budget tag = 0 (None), stop_reason tag = 0 (Converged)
    0x00, 0x00, // diagnostics count = 0, artifacts count = 0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Known offsets into the fixture layouts, locked by assertions below.
const MIN_RUN_KIND_OFFSET: usize = 83;
const MIN_PRECISION_ABS_OFFSET: usize = 88;
const MIN_STOP_OFFSET: usize = 130;
const MIN_ARTIFACT_COUNT_OFFSET: usize = 135;
// Carrier layout (one 4-byte parameter name "flag", one target value).
const CARRIER_CONFIG_TAG_OFFSET: usize = 96;
const CARRIER_LOGICAL_OFFSET: usize = 97;
const CARRIER_TARGET_TAG_OFFSET: usize = 114;
const CARRIER_STOP_OFFSET: usize = 152;
const CARRIER_LEN: usize = 161;

fn carrier_bytes() -> Vec<u8> {
    // Base stop is Converged (tag 0) so the locked offsets are stable.
    encode_run_manifest(&carrier_manifest_with_stop(StopReason::Converged)).unwrap()
}

fn lock_carrier_offsets(bytes: &[u8]) {
    assert_eq!(bytes.len(), CARRIER_LEN, "carrier layout length");
    assert_eq!(bytes[MIN_RUN_KIND_OFFSET], 0);
    assert_eq!(bytes[CARRIER_CONFIG_TAG_OFFSET], 3, "Logical tag");
    assert_eq!(bytes[CARRIER_LOGICAL_OFFSET], 1, "Logical payload true");
    assert_eq!(bytes[CARRIER_TARGET_TAG_OFFSET], 1, "target Some tag");
    assert_eq!(bytes[CARRIER_STOP_OFFSET], 0, "Converged tag");
}

// ---------------------------------------------------------------------------
// DoD: schema is versioned.
// ---------------------------------------------------------------------------

#[test]
fn t_manifest_version_constant_is_one() {
    assert_eq!(MANIFEST_VERSION, 1);
    assert_eq!(FINGERPRINT_ALGORITHM_SHA256_V1, "sha256-v1");
}

#[test]
fn t_golden_manifest_full() {
    let manifest = full_manifest();
    let bytes = encode_run_manifest(&manifest).unwrap();
    assert_eq!(bytes.as_slice(), FULL_GOLDEN.as_slice());

    let decoded = decode_run_manifest(&FULL_GOLDEN).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);

    // Spot-check the decoded accessors.
    assert_eq!(
        decoded.run_kind().seed(),
        Some(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10)
    );
    assert_eq!(decoded.params().len(), 4);
    assert_eq!(decoded.artifacts().len(), 2);
    assert_eq!(decoded.target().map(<[ValidFloat]>::len), Some(2));
}

#[test]
fn t_golden_manifest_minimal() {
    let manifest = minimal_manifest();
    let bytes = encode_run_manifest(&manifest).unwrap();
    assert_eq!(bytes.as_slice(), MINIMAL_GOLDEN.as_slice());

    let decoded = decode_run_manifest(&MINIMAL_GOLDEN).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);
}

// ---------------------------------------------------------------------------
// DoD: deterministic calls distinguished from stochastic sessions.
// ---------------------------------------------------------------------------

#[test]
fn t_deterministic_carries_no_seed() {
    let deterministic = minimal_manifest();
    assert!(!deterministic.run_kind().is_stochastic());
    assert_eq!(deterministic.run_kind().seed(), None);
    assert!(deterministic.run_kind().rng().is_none());

    // The stochastic form differs only in its RunKind encoding. The stochastic
    // RunKind is 41 bytes = 1 tag + 16 seed + (4+15) + (4+1); the deterministic
    // RunKind is a single tag byte, so the whole-manifest length delta is 40.
    let stochastic = stochastic_minimal_manifest();
    let deterministic_bytes = encode_run_manifest(&deterministic).unwrap();
    let stochastic_bytes = encode_run_manifest(&stochastic).unwrap();
    let delta = stochastic_bytes.len() - deterministic_bytes.len();
    assert_eq!(delta, 40, "measured delta; stochastic RunKind is 41 bytes");
    assert_eq!(
        delta + 1,
        1 + 16 + (4 + 15) + (4 + 1),
        "stochastic RunKind encoding size accounts for the +1 tag delta"
    );
}

#[test]
fn t_stochastic_round_trips_seed() {
    let manifest = RunManifest::new(minimal_params(
        RunKind::stochastic(u128::MAX, "xoshiro256pp-v1".to_string(), "1".to_string()).unwrap(),
    ))
    .unwrap();
    assert!(manifest.run_kind().is_stochastic());
    assert_eq!(manifest.run_kind().seed(), Some(u128::MAX));
    let rng = manifest.run_kind().rng().unwrap();
    assert_eq!(rng.algorithm(), "xoshiro256pp-v1");
    assert_eq!(rng.version(), "1");

    let bytes = encode_run_manifest(&manifest).unwrap();
    let decoded = decode_run_manifest(&bytes).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);
}

#[test]
fn t_stochastic_empty_rng_rejected() {
    let error = RunKind::stochastic(1, String::new(), "1".to_string()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);

    let error = RunKind::stochastic(1, "xoshiro256pp-v1".to_string(), String::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);
}

#[test]
fn t_deterministic_with_seed_bytes_is_trailing() {
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes.extend_from_slice(&[0u8; 16]);
    let error = decode_run_manifest(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TrailingPayload);
}

#[test]
fn t_stochastic_cut_before_seed_is_truncated() {
    let bytes = encode_run_manifest(&stochastic_minimal_manifest()).unwrap();
    // The stochastic RunKind tag sits at the minimal run-kind offset; a single
    // seed byte follows it before the value is cut off.
    let truncated = &bytes[..MIN_RUN_KIND_OFFSET + 2];
    let error = decode_run_manifest(truncated).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TruncatedPayload);
}

// ---------------------------------------------------------------------------
// DoD: unknown/failed/aborted terminal states are explicit.
// ---------------------------------------------------------------------------

#[test]
fn t_stop_variants_golden() {
    let bytes = carrier_bytes();
    lock_carrier_offsets(&bytes);

    for (tag, stop_reason) in stop_variants().into_iter().enumerate() {
        let manifest = carrier_manifest_with_stop(stop_reason);
        let encoded = encode_run_manifest(&manifest).unwrap();
        assert_eq!(
            encoded[CARRIER_STOP_OFFSET],
            u8::try_from(tag).unwrap(),
            "stop-reason tag byte"
        );
        let decoded = decode_run_manifest(&encoded).unwrap();
        assert_eq!(decoded.stop_reason(), stop_reason);
        assert_eq!(encode_run_manifest(&decoded).unwrap(), encoded);
    }
}

#[test]
fn t_stop_classification() {
    for reason in [
        StopReason::Converged,
        StopReason::TargetAchieved,
        StopReason::EvaluationBudgetExhausted,
        StopReason::IterationBudgetExhausted,
        StopReason::LogicalDeadlineReached { ticks: 0 },
    ] {
        assert!(reason.is_normal_completion(), "{reason:?}");
        assert!(!reason.is_abnormal_termination(), "{reason:?}");
    }
    for reason in [StopReason::Aborted, StopReason::Failed, StopReason::Unknown] {
        assert!(!reason.is_normal_completion(), "{reason:?}");
        assert!(reason.is_abnormal_termination(), "{reason:?}");
    }
}

#[test]
fn t_deadline_ticks_round_trip() {
    let manifest =
        carrier_manifest_with_stop(StopReason::LogicalDeadlineReached { ticks: u64::MAX });
    let bytes = encode_run_manifest(&manifest).unwrap();
    let decoded = decode_run_manifest(&bytes).unwrap();
    assert_eq!(
        decoded.stop_reason(),
        StopReason::LogicalDeadlineReached { ticks: u64::MAX }
    );
    assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);
}

// ---------------------------------------------------------------------------
// Round-trip and schema-negative tests.
// ---------------------------------------------------------------------------

#[test]
fn t_roundtrip_byte_identity() {
    let full = full_manifest();
    let full_bytes = encode_run_manifest(&full).unwrap();
    assert_eq!(decode_run_manifest(&full_bytes).unwrap(), full);
    assert_eq!(
        encode_run_manifest(&decode_run_manifest(&full_bytes).unwrap()).unwrap(),
        full_bytes
    );

    let minimal = minimal_manifest();
    let minimal_bytes = encode_run_manifest(&minimal).unwrap();
    assert_eq!(decode_run_manifest(&minimal_bytes).unwrap(), minimal);
    assert_eq!(
        encode_run_manifest(&decode_run_manifest(&minimal_bytes).unwrap()).unwrap(),
        minimal_bytes
    );

    for stop_reason in stop_variants() {
        let manifest = carrier_manifest_with_stop(stop_reason);
        let bytes = encode_run_manifest(&manifest).unwrap();
        let decoded = decode_run_manifest(&bytes).unwrap();
        assert_eq!(decoded, manifest);
        assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);
    }
}

#[test]
fn t_wrong_schema_header_rejected() {
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[0] = 0x02;
    let error = decode_run_manifest(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnsupportedSchemaVersion);
}

#[test]
fn t_wrong_manifest_version_rejected() {
    for version in [2u32, 0u32] {
        let mut bytes = MINIMAL_GOLDEN.to_vec();
        bytes[4..8].copy_from_slice(&version.to_le_bytes());
        let error = decode_run_manifest(&bytes).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnsupportedSchemaVersion);
    }
}

#[test]
fn t_truncated_rejected() {
    let truncated = &MINIMAL_GOLDEN[..MINIMAL_GOLDEN.len() - 1];
    assert_eq!(
        decode_run_manifest(truncated).unwrap_err().kind(),
        ErrorKind::TruncatedPayload
    );
    assert_eq!(
        decode_run_manifest(&[]).unwrap_err().kind(),
        ErrorKind::TruncatedPayload
    );
}

#[test]
fn t_digest_truncated_inside() {
    // The fingerprint digest occupies MINIMAL_GOLDEN[37..69] (after the 4-byte
    // algorithm length @24..28 and the 9-byte "sha256-v1" content @28..37). A
    // cut at the digest start, in its middle, or on its final byte must all
    // report TruncatedPayload rather than panic.
    assert_eq!(
        &MINIMAL_GOLDEN[37..69],
        &[0u8; 32],
        "digest slice is all-zero"
    );
    for cut in [37usize, 50, 68] {
        assert_eq!(
            decode_run_manifest(&MINIMAL_GOLDEN[..cut])
                .unwrap_err()
                .kind(),
            ErrorKind::TruncatedPayload,
            "cut inside fingerprint digest at {cut}"
        );
    }

    // The first artifact digest is FULL_GOLDEN[339..371] (0x11 * 32).
    let artifact_digest = 339usize;
    assert_eq!(
        &FULL_GOLDEN[artifact_digest..artifact_digest + 32],
        &[0x11u8; 32],
        "first artifact digest slice"
    );
    let cut = artifact_digest + 16; // middle of the 32-byte artifact digest
    assert_eq!(
        decode_run_manifest(&FULL_GOLDEN[..cut]).unwrap_err().kind(),
        ErrorKind::TruncatedPayload,
        "cut inside first artifact digest"
    );
}

#[test]
fn t_trailing_rejected() {
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes.push(0x00);
    let error = decode_run_manifest(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TrailingPayload);
}

#[test]
fn t_unknown_tags_rejected() {
    // run-kind tag -> 2
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[MIN_RUN_KIND_OFFSET] = 2;
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // stop tag -> 8
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[MIN_STOP_OFFSET] = 8;
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // config-value tag -> 4
    let mut bytes = carrier_bytes();
    bytes[CARRIER_CONFIG_TAG_OFFSET] = 4;
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // logical payload byte -> 2
    let mut bytes = carrier_bytes();
    bytes[CARRIER_LOGICAL_OFFSET] = 2;
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // target option tag -> 2
    let mut bytes = carrier_bytes();
    bytes[CARRIER_TARGET_TAG_OFFSET] = 2;
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );
}

#[test]
fn t_nan_bits_rejected() {
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[MIN_PRECISION_ABS_OFFSET..MIN_PRECISION_ABS_OFFSET + 8]
        .copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    let error = decode_run_manifest(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidNotFinite);
}

#[test]
fn t_empty_solvers_rejected() {
    // Wire: solver_versions count patched to 0.
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[69..73].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );

    // Unit: validating constructor rejects an empty solver list.
    let mut params = minimal_params(RunKind::deterministic());
    params.solver_versions = Vec::new();
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn t_empty_target_some_rejected() {
    let mut params = minimal_params(RunKind::deterministic());
    params.target = Some(Vec::new());
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn t_duplicate_names_rejected() {
    // Duplicate solver ids.
    let mut params = minimal_params(RunKind::deterministic());
    params.solver_versions = vec![solver("dup", "1"), solver("dup", "2")];
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // Case-different names are allowed (case-sensitive uniqueness).
    let mut params = minimal_params(RunKind::deterministic());
    params.solver_versions = vec![solver("A", "1"), solver("a", "2")];
    assert!(RunManifest::new(params).is_ok());

    // Duplicate config names.
    let mut params = minimal_params(RunKind::deterministic());
    params.params = vec![
        ConfigParam::new("p".to_string(), ConfigValue::Logical(true)).unwrap(),
        ConfigParam::new("p".to_string(), ConfigValue::Logical(false)).unwrap(),
    ];
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // Duplicate artifact names.
    let mut params = minimal_params(RunKind::deterministic());
    params.artifacts = vec![
        ArtifactHash::new("a".to_string(), "sha256-v1".to_string(), [0; 32]).unwrap(),
        ArtifactHash::new("a".to_string(), "sha256-v1".to_string(), [1; 32]).unwrap(),
    ];
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::Reserved
    );
}

#[test]
fn t_duplicate_solver_ids_rejected_on_decode() {
    // Unit invariant: two solver entries sharing an id are rejected at
    // construction time.
    let mut params = minimal_params(RunKind::deterministic());
    params.solver_versions = vec![solver("dup", "1"), solver("dup", "2")];
    assert_eq!(
        RunManifest::new(params).unwrap_err().kind(),
        ErrorKind::Reserved
    );

    // Wire path: FULL_GOLDEN stores solver-a id "solver-a" @[77..85] and
    // solver-b id "solver-b" @[98..106] (both 8-byte contents behind unchanged
    // 4-byte length prefixes). Copying solver-b's content over solver-a's makes
    // the two ids identical, and the decoder must rebuild through
    // RunManifest::new and reject the duplicate.
    let mut bytes = FULL_GOLDEN.to_vec();
    assert_eq!(&bytes[77..85], b"solver-a", "solver-a id content");
    assert_eq!(&bytes[98..106], b"solver-b", "solver-b id content");
    let duplicate: [u8; 8] = bytes[98..106].try_into().unwrap();
    bytes[77..85].copy_from_slice(&duplicate);
    assert_eq!(&bytes[77..85], b"solver-b", "both solver ids now equal");
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );
}

#[test]
fn t_invalid_utf8_rejected() {
    // The fingerprint algorithm "sha256-v1" content occupies
    // MINIMAL_GOLDEN[28..37] (length prefix @24..28).
    assert_eq!(
        &MINIMAL_GOLDEN[28..37],
        FINGERPRINT_ALGORITHM_SHA256_V1.as_bytes(),
        "algorithm content slice"
    );
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[28] = 0xFF; // invalid UTF-8 continuation byte inside the string
    assert_eq!(
        decode_run_manifest(&bytes).unwrap_err().kind(),
        ErrorKind::Reserved
    );
}

#[test]
fn t_empty_strings_rejected() {
    // Empty solver id and version.
    assert_eq!(
        SolverVersion::new(String::new(), "1".to_string())
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        SolverVersion::new("s".to_string(), String::new())
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    // Empty artifact name (and algorithm).
    assert_eq!(
        ArtifactHash::new(String::new(), "sha256-v1".to_string(), [0; 32])
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        ArtifactHash::new("a".to_string(), String::new(), [0; 32])
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    // Empty config name.
    assert_eq!(
        ConfigParam::new(String::new(), ConfigValue::Logical(true))
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn t_huge_count_is_truncated_not_panic() {
    let mut bytes = MINIMAL_GOLDEN.to_vec();
    bytes[MIN_ARTIFACT_COUNT_OFFSET..MIN_ARTIFACT_COUNT_OFFSET + 4]
        .copy_from_slice(&u32::MAX.to_le_bytes());
    let error = decode_run_manifest(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TruncatedPayload);
}

#[test]
fn t_negative_precision_rejected() {
    assert_eq!(
        Precision::new(vf(-0.1), vf(0.0)).unwrap_err().kind(),
        ErrorKind::InvertedBounds
    );
    assert_eq!(
        Precision::new(vf(0.0), vf(-1.0)).unwrap_err().kind(),
        ErrorKind::InvertedBounds
    );
    assert!(Precision::new(vf(0.0), vf(0.0)).is_ok());
}

#[test]
fn t_objective_descriptors_require_non_empty() {
    let empty = encode_objective_descriptors(&[]).unwrap();
    assert_eq!(
        decode_objective_descriptors(&empty).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );

    // Constraint descriptors may be empty and round-trip to an empty vec.
    let empty_constraints = encode_constraint_descriptors(&[]).unwrap();
    assert_eq!(
        decode_constraint_descriptors(&empty_constraints).unwrap(),
        Vec::new()
    );
}

#[test]
fn t_descriptor_helpers_round_trip() {
    let objectives = vec![
        ObjectiveDescriptor::new(0, ObjectiveSense::Minimize),
        ObjectiveDescriptor::new(1, ObjectiveSense::Maximize),
    ];
    let encoded = encode_objective_descriptors(&objectives).unwrap();
    let decoded = decode_objective_descriptors(&encoded).unwrap();
    assert_eq!(decoded, objectives);
    assert_eq!(encode_objective_descriptors(&decoded).unwrap(), encoded);

    let constraints = vec![
        ConstraintDescriptor::new(0, ConstraintKind::Hard).unwrap(),
        ConstraintDescriptor::new(1, ConstraintKind::soft(vf(0.25)).unwrap()).unwrap(),
    ];
    let encoded = encode_constraint_descriptors(&constraints).unwrap();
    let decoded = decode_constraint_descriptors(&encoded).unwrap();
    assert_eq!(decoded, constraints);
    assert_eq!(encode_constraint_descriptors(&decoded).unwrap(), encoded);
}

// ---------------------------------------------------------------------------
// Fingerprint / SHA-256.
// ---------------------------------------------------------------------------

#[test]
fn t_fingerprint_deterministic() {
    let (variables, objectives, constraints) = problem();
    let first = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    let second = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    assert_eq!(first, second);
}

#[test]
fn t_fingerprint_pinned_against_independent_oracle() {
    // Pinned digest re-derived by an independent verifier (and re-checked here
    // via SHA-256 over the exact 128-byte canonical stream built by
    // `problem()`): "MAMUT-MATH-PROBLEM-V1" + 3 variables + 2 objectives +
    // 2 constraints. Any drift in the canonicalization breaks this assertion
    // instead of silently changing every recorded fingerprint.
    let (variables, objectives, constraints) = problem();
    let fingerprint = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    assert_eq!(fingerprint.algorithm(), FINGERPRINT_ALGORITHM_SHA256_V1);
    assert_eq!(
        hex32(fingerprint.digest()),
        "9a30cccb7678ff43ba8adc79db938446d528b3f70b2b1d0f402045897fb78808"
    );
}

#[test]
fn t_fingerprint_sensitive() {
    let (variables, objectives, constraints) = problem();
    let base = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();

    // Bound change 10.0 -> 10.25.
    let mut changed_bound = variables.clone();
    changed_bound[0] = Variable::new(
        VariableId(0),
        VariableKind::continuous(vf(0.0), vf(10.25)).unwrap(),
    )
    .unwrap();
    let bound = Fingerprint::for_problem(&changed_bound, &objectives, &constraints).unwrap();

    // Flipped objective sense.
    let mut flipped = objectives.clone();
    flipped[0] = ObjectiveDescriptor::new(0, ObjectiveSense::Maximize);
    let sense = Fingerprint::for_problem(&variables, &flipped, &constraints).unwrap();

    // Changed soft tolerance.
    let mut changed_tolerance = constraints.clone();
    changed_tolerance[1] =
        ConstraintDescriptor::new(1, ConstraintKind::soft(vf(0.2)).unwrap()).unwrap();
    let tolerance = Fingerprint::for_problem(&variables, &objectives, &changed_tolerance).unwrap();

    assert_ne!(base, bound);
    assert_ne!(base, sense);
    assert_ne!(base, tolerance);
    assert_ne!(bound, sense);
    assert_ne!(bound, tolerance);
    assert_ne!(sense, tolerance);
}

#[test]
fn t_fingerprint_order_sensitive() {
    let (mut variables, objectives, constraints) = problem();
    let original = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    variables.swap(0, 1);
    let swapped = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    assert_ne!(original, swapped);
}

#[test]
fn t_fingerprint_algorithm_tag() {
    let (variables, objectives, constraints) = problem();
    let fingerprint = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();
    assert_eq!(fingerprint.algorithm(), "sha256-v1");
    assert_eq!(fingerprint.algorithm(), FINGERPRINT_ALGORITHM_SHA256_V1);
}

#[test]
fn t_artifact_sha256_known_answer() {
    let artifact = ArtifactHash::sha256("a".to_string(), b"abc").unwrap();
    assert_eq!(artifact.algorithm(), "sha256-v1");
    assert_eq!(
        hex32(artifact.digest()),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

// ---------------------------------------------------------------------------
// Canonical JSON.
// ---------------------------------------------------------------------------

#[test]
fn t_export_golden_string() {
    let manifest = RunManifest::new(RunManifestParams {
        manifest_version: MANIFEST_VERSION,
        run_id: RunId::new(7),
        fingerprint: Fingerprint::new("sha256-v1".to_string(), [0u8; 32]).unwrap(),
        solver_versions: vec![solver("nsga-iii", "0.1.0")],
        run_kind: RunKind::deterministic(),
        params: Vec::new(),
        precision: Precision::new(vf(1e-9), vf(1e-6)).unwrap(),
        target: None,
        evaluation: EvaluationRecord::new(100, 2, 100, Some(200)),
        stop_reason: StopReason::Converged,
        diagnostics: Diagnostics::new(vec![diag(DiagnosticCode::EvaluationWarning, "ok")]),
        artifacts: Vec::new(),
    })
    .unwrap();

    let expected = format!(
        "{{\"manifest_version\":1,\"run_id\":\"7\",\"fingerprint\":{{\"algorithm\":\"sha256-v1\",\
\"digest\":\"{}\"}},\"solver_versions\":[{{\"id\":\"nsga-iii\",\"version\":\"0.1.0\"}}],\
\"run_kind\":{{\"kind\":\"deterministic\"}},\"params\":[],\"precision\":{{\"absolute_tolerance\":\
\"0x{:016x}\",\"relative_tolerance\":\"0x{:016x}\"}},\"target\":null,\"evaluation\":{{\
\"completed_evaluations\":100,\"failed_evaluations\":2,\"logical_ticks_used\":100,\
\"logical_tick_budget\":200}},\"stop_reason\":{{\"reason\":\"converged\"}},\"diagnostics\":[{{\
\"code\":\"evaluation_warning\",\"note\":\"ok\"}}],\"artifacts\":[]}}",
        "0".repeat(64),
        (1e-9_f64).to_bits(),
        (1e-6_f64).to_bits(),
    );

    assert_eq!(manifest.export().unwrap(), expected);

    // The float hex literals are verified independently of to_bits().
    assert_eq!(format!("{:016x}", (1e-9_f64).to_bits()), "3e112e0be826d695");
    assert_eq!(format!("{:016x}", (1e-6_f64).to_bits()), "3eb0c6f7a0b5ed8d");
    assert!(expected.contains("\"absolute_tolerance\":\"0x3e112e0be826d695\""));
    assert!(expected.contains("\"relative_tolerance\":\"0x3eb0c6f7a0b5ed8d\""));
}

#[test]
fn t_export_escaping() {
    let note = "a\"b\\c\u{0001}d\u{001f}".to_string();
    let mut params = minimal_params(RunKind::deterministic());
    params.diagnostics = Diagnostics::new(vec![Diagnostic {
        code: DiagnosticCode::EvaluationWarning,
        note,
    }]);
    let manifest = RunManifest::new(params).unwrap();

    let exported = manifest.export().unwrap();

    // `\"` for the quote, `\\` for the backslash, `\u0001`/`\u001f` for the
    // controls; the short form `\n` must never appear.
    assert!(
        exported.contains("a\\\"b\\\\c\\u0001d\\u001f"),
        "unexpected escaping: {exported}"
    );
    assert!(exported.contains("\\u0001"));
    assert!(exported.contains("\\u001f"));
    assert!(
        !exported.contains("\\n"),
        "no short-form escapes: {exported}"
    );
}

#[test]
fn t_export_float_hex() {
    let mut params = minimal_params(RunKind::deterministic());
    params.params = vec![ConfigParam::new("x".to_string(), ConfigValue::Float(vf(2.5))).unwrap()];
    let manifest = RunManifest::new(params).unwrap();
    let exported = manifest.export().unwrap();
    assert!(
        exported.contains("\"value\":{\"type\":\"float\",\"value\":\"0x4004000000000000\"}"),
        "unexpected float encoding: {exported}"
    );
}

#[test]
fn t_export_has_no_problem_payload() {
    let exported = minimal_manifest().export().unwrap();
    assert!(exported.contains("\"fingerprint\""));
    assert!(!exported.contains("variables"));
}

// ---------------------------------------------------------------------------
// Hydraulics example (test-only, opaque metadata strings).
// ---------------------------------------------------------------------------

#[test]
fn t_hydraulics_example_manifest_is_backend_neutral() {
    let (variables, objectives, constraints) = problem();
    let fingerprint = Fingerprint::for_problem(&variables, &objectives, &constraints).unwrap();

    let manifest = RunManifest::new(RunManifestParams {
        manifest_version: MANIFEST_VERSION,
        run_id: RunId::new(0xDEAD_BEEF_0000_0001),
        fingerprint,
        solver_versions: vec![
            solver("hydraulics-optimizer", "2.4.1"),
            solver("hydraulics-backend", "0.9.0"),
        ],
        run_kind: RunKind::stochastic(
            0x9E37_79B9_7F4A_7C15_1234_5678_90AB_CDEF,
            "xoshiro256pp-v1".to_string(),
            "1".to_string(),
        )
        .unwrap(),
        params: vec![
            ConfigParam::new(
                "network_label".to_string(),
                ConfigValue::Text("looped-12".to_string()),
            )
            .unwrap(),
            ConfigParam::new(
                "pump_label".to_string(),
                ConfigValue::Text("p-03".to_string()),
            )
            .unwrap(),
            ConfigParam::new("demand_cases".to_string(), ConfigValue::Integer(4)).unwrap(),
            ConfigParam::new("use_cache".to_string(), ConfigValue::Logical(true)).unwrap(),
        ],
        precision: Precision::new(vf(1e-9), vf(1e-6)).unwrap(),
        target: Some(vec![vf(41.5)]),
        evaluation: EvaluationRecord::new(512, 3, 512, Some(1024)),
        stop_reason: StopReason::EvaluationBudgetExhausted,
        diagnostics: Diagnostics::new(vec![diag(
            DiagnosticCode::EvaluationWarning,
            "converged within tolerance",
        )]),
        artifacts: vec![ArtifactHash::sha256("heads-table".to_string(), b"h,0.5\n").unwrap()],
    })
    .unwrap();

    let bytes = encode_run_manifest(&manifest).unwrap();
    let decoded = decode_run_manifest(&bytes).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(encode_run_manifest(&decoded).unwrap(), bytes);

    let exported = decoded.export().unwrap();
    assert!(exported.contains("hydraulics-optimizer"));
    assert!(exported.contains("heads-table"));
}

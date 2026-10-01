//! DoD (4): canonical round-trips and pinned golden byte fixtures.

use mamut_math_core::{
    decode_candidate, decode_constraint_violations, decode_diagnostics, decode_evaluation_outcome,
    decode_feasibility, decode_objective_values, decode_variables, encode_candidate,
    encode_constraint_violations, encode_diagnostics, encode_evaluation_outcome,
    encode_feasibility, encode_objective_values, encode_variables, Candidate, ConstraintViolation,
    Diagnostic, DiagnosticCode, Diagnostics, ErrorKind, EvaluationOutcome, Feasibility,
    ObjectiveValues, ValidFloat, Variable, VariableId, VariableKind,
};

fn vf(value: f64) -> ValidFloat {
    ValidFloat::new(value).expect("finite literal")
}

fn continuous_variable(id: u64, lower: f64, upper: f64) -> Variable {
    Variable::new(
        VariableId(id),
        VariableKind::continuous(vf(lower), vf(upper)).unwrap(),
    )
    .unwrap()
}

fn categorical_variable(id: u64, cardinality: u64) -> Variable {
    Variable::new(
        VariableId(id),
        VariableKind::categorical(cardinality).unwrap(),
    )
    .unwrap()
}

fn candidate() -> Candidate {
    Candidate::new(vec![vf(1.0), vf(2.0), vf(3.5)]).unwrap()
}

fn success_outcome() -> EvaluationOutcome {
    EvaluationOutcome::Success {
        objectives: ObjectiveValues::new(vec![vf(10.0)]).unwrap(),
        constraints: vec![ConstraintViolation::new(vf(0.5)).unwrap()],
        feasibility: Feasibility::SoftViolation { violated: vec![0] },
        diagnostics: Diagnostics::new(Vec::new()),
    }
}

fn failure_outcome() -> EvaluationOutcome {
    EvaluationOutcome::Failure {
        error: mamut_math_core::MathError::new(ErrorKind::DomainMismatch, "bad"),
    }
}

// ---------------------------------------------------------------------------
// Golden fixtures (SCHEMA_VERSION = 1). Each array is the exact expected
// canonical byte stream; the comments annotate every field in order.
// ---------------------------------------------------------------------------

const CONTINUOUS_GOLDEN: [u8; 33] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1 (u32 LE)
    0x01, 0x00, 0x00, 0x00, // variable count = 1 (u32 LE)
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // VariableId(1) (u64 LE)
    0x00, // VariableKind::Continuous tag
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // lower = 0.0 (f64 bits LE)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, // upper = 1.0 (f64 bits LE)
];

const CONTINUOUS_NEGATIVE_GOLDEN: [u8; 33] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1 (u32 LE)
    0x01, 0x00, 0x00, 0x00, // variable count = 1 (u32 LE)
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // VariableId(5) (u64 LE)
    0x00, // VariableKind::Continuous tag
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0xBF, // lower = -1.5 (0xBFF8000000000000)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F, // upper = 1.5 (0x3FF8000000000000)
];

const CATEGORICAL_GOLDEN: [u8; 25] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1
    0x01, 0x00, 0x00, 0x00, // variable count = 1
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // VariableId(2)
    0x03, // VariableKind::Categorical tag
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cardinality = 3 (u64 LE)
];

const CANDIDATE_GOLDEN: [u8; 32] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1
    0x03, 0x00, 0x00, 0x00, // value count = 3
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, // 1.0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, // 2.0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x40, // 3.5
];

const SUCCESS_GOLDEN: [u8; 42] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1
    0x00, // EvaluationOutcome::Success tag
    0x01, 0x00, 0x00, 0x00, // objectives count = 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x40, // objective = 10.0
    0x01, 0x00, 0x00, 0x00, // constraints count = 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x3F, // violation magnitude = 0.5
    0x02, // Feasibility::SoftViolation tag
    0x01, 0x00, 0x00, 0x00, // violated count = 1
    0x00, 0x00, 0x00, 0x00, // violated[0] = 0
    0x00, 0x00, 0x00, 0x00, // diagnostics count = 0
];

const FAILURE_GOLDEN: [u8; 17] = [
    0x01, 0x00, 0x00, 0x00, // SCHEMA_VERSION = 1
    0x01, // EvaluationOutcome::Failure tag
    0x04, // ErrorKind::DomainMismatch tag
    0x03, 0x00, 0x00, 0x00, // message byte length = 3
    0x62, 0x61, 0x64, // "bad"
    0x00, 0x00, 0x00, 0x00, // context count = 0
];

// ---------------------------------------------------------------------------
// Golden fixture assertions
// ---------------------------------------------------------------------------

#[test]
fn golden_continuous_variable() {
    let bytes = encode_variables(&[continuous_variable(1, 0.0, 1.0)]).unwrap();
    assert_eq!(bytes.as_slice(), CONTINUOUS_GOLDEN.as_slice());
}

#[test]
fn golden_continuous_negative_variable() {
    let variable = continuous_variable(5, -1.5, 1.5);
    let bytes = encode_variables(std::slice::from_ref(&variable)).unwrap();
    assert_eq!(bytes.as_slice(), CONTINUOUS_NEGATIVE_GOLDEN.as_slice());

    let decoded = decode_variables(&CONTINUOUS_NEGATIVE_GOLDEN).unwrap();
    assert_eq!(decoded, vec![variable]);
}

#[test]
fn golden_categorical_variable() {
    let bytes = encode_variables(&[categorical_variable(2, 3)]).unwrap();
    assert_eq!(bytes.as_slice(), CATEGORICAL_GOLDEN.as_slice());
}

#[test]
fn golden_three_value_candidate() {
    let bytes = encode_candidate(&candidate()).unwrap();
    assert_eq!(bytes.as_slice(), CANDIDATE_GOLDEN.as_slice());
}

#[test]
fn golden_success_with_soft_violation() {
    let bytes = encode_evaluation_outcome(&success_outcome()).unwrap();
    assert_eq!(bytes.as_slice(), SUCCESS_GOLDEN.as_slice());
}

#[test]
fn golden_failure() {
    let bytes = encode_evaluation_outcome(&failure_outcome()).unwrap();
    assert_eq!(bytes.as_slice(), FAILURE_GOLDEN.as_slice());
}

// ---------------------------------------------------------------------------
// Corrupted-payload behavior
// ---------------------------------------------------------------------------

#[test]
fn flipped_version_is_unsupported() {
    let mut bytes = CANDIDATE_GOLDEN.to_vec();
    bytes[0] = 0x02;
    let error = decode_candidate(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnsupportedSchemaVersion);
}

#[test]
fn dropped_trailing_byte_is_truncated() {
    let truncated = &CANDIDATE_GOLDEN[..CANDIDATE_GOLDEN.len() - 1];
    let error = decode_candidate(truncated).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TruncatedPayload);
}

#[test]
fn appended_byte_is_trailing() {
    let mut bytes = CANDIDATE_GOLDEN.to_vec();
    bytes.push(0x00);
    let error = decode_candidate(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TrailingPayload);
}

#[test]
fn nan_bit_pattern_is_invalid_not_finite() {
    let mut bytes = CANDIDATE_GOLDEN.to_vec();
    // The first payload float occupies bytes 8..16; overwrite it with NaN.
    bytes[8..16].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    let error = decode_candidate(&bytes).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidNotFinite);
}

#[test]
fn empty_payload_is_truncated() {
    let error = decode_candidate(&[]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TruncatedPayload);
}

// ---------------------------------------------------------------------------
// encode -> decode -> encode is byte-identical for every type
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_variables() {
    let original = vec![
        continuous_variable(1, 0.0, 1.0),
        Variable::new(VariableId(2), VariableKind::integer(-4, 4).unwrap()).unwrap(),
        Variable::new(
            VariableId(3),
            VariableKind::discrete(
                mamut_math_core::DiscreteSet::new(vec![vf(1.0), vf(2.0), vf(3.0)]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        categorical_variable(4, 5),
    ];
    let bytes = encode_variables(&original).unwrap();
    let decoded = decode_variables(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_variables(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_candidate() {
    let original = candidate();
    let bytes = encode_candidate(&original).unwrap();
    let decoded = decode_candidate(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_candidate(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_objective_values() {
    let original = ObjectiveValues::new(vec![vf(-1.25), vf(0.0), vf(1e300)]).unwrap();
    let bytes = encode_objective_values(&original).unwrap();
    let decoded = decode_objective_values(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_objective_values(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_diagnostics() {
    let original = Diagnostics::new(vec![
        Diagnostic {
            code: DiagnosticCode::DegenerateFront,
            note: "single point front".to_string(),
        },
        Diagnostic {
            code: DiagnosticCode::Reserved,
            note: String::new(),
        },
    ]);
    let bytes = encode_diagnostics(&original).unwrap();
    let decoded = decode_diagnostics(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_diagnostics(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_feasibility() {
    for original in [
        Feasibility::Feasible,
        Feasibility::InfeasibleHard {
            violated: vec![0, 3, 7],
        },
        Feasibility::SoftViolation { violated: vec![2] },
    ] {
        let bytes = encode_feasibility(&original).unwrap();
        let decoded = decode_feasibility(&bytes).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(encode_feasibility(&decoded).unwrap(), bytes);
    }
}

#[test]
fn roundtrip_constraint_violations() {
    let original = vec![
        ConstraintViolation::new(vf(0.0)).unwrap(),
        ConstraintViolation::new(vf(0.25)).unwrap(),
    ];
    let bytes = encode_constraint_violations(&original).unwrap();
    let decoded = decode_constraint_violations(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_constraint_violations(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_evaluation_outcome_success() {
    let original = success_outcome();
    let bytes = encode_evaluation_outcome(&original).unwrap();
    let decoded = decode_evaluation_outcome(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_evaluation_outcome(&decoded).unwrap(), bytes);
}

#[test]
fn roundtrip_evaluation_outcome_failure() {
    let original = failure_outcome();
    let bytes = encode_evaluation_outcome(&original).unwrap();
    let decoded = decode_evaluation_outcome(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_evaluation_outcome(&decoded).unwrap(), bytes);
}

// ---------------------------------------------------------------------------
// Wire-format bounds: index values are `u32`; larger values must fail, not truncate.
// ---------------------------------------------------------------------------

#[test]
fn feasibility_index_at_u32_max_round_trips() {
    let original = Feasibility::InfeasibleHard {
        violated: vec![u32::MAX as usize],
    };
    let bytes = encode_feasibility(&original).unwrap();
    let decoded = decode_feasibility(&bytes).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(encode_feasibility(&decoded).unwrap(), bytes);
}

#[test]
fn feasibility_index_above_u32_max_is_rejected() {
    let oversized = Feasibility::InfeasibleHard {
        violated: vec![u32::MAX as usize + 1],
    };
    let error = encode_feasibility(&oversized).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::IndexOutOfRange);
}

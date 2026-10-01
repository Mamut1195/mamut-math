//! DoD (1)–(3): mixed-variable representation, rejection matrix and feasibility.

use mamut_math_core::{
    decode_evaluation_outcome, encode_evaluation_outcome, evaluate_feasibility, Candidate,
    ConstraintDescriptor, ConstraintKind, ConstraintViolation, DiscreteSet, ErrorKind,
    EvaluationOutcome, Feasibility, MathError, ObjectiveValues, ValidFloat, Variable, VariableId,
    VariableKind,
};

fn vf(value: f64) -> ValidFloat {
    ValidFloat::new(value).expect("finite literal")
}

fn continuous(lower: f64, upper: f64) -> VariableKind {
    VariableKind::continuous(vf(lower), vf(upper)).expect("valid continuous bounds")
}

fn mixed_problem() -> Vec<Variable> {
    vec![
        Variable::new(VariableId(0), continuous(0.0, 10.0)).unwrap(),
        Variable::new(VariableId(1), continuous(-5.0, 5.0)).unwrap(),
        Variable::new(VariableId(2), VariableKind::integer(0, 100).unwrap()).unwrap(),
        Variable::new(
            VariableId(3),
            VariableKind::discrete(DiscreteSet::new(vec![vf(0.5), vf(1.5), vf(2.5)]).unwrap())
                .unwrap(),
        )
        .unwrap(),
        Variable::new(VariableId(4), VariableKind::categorical(3).unwrap()).unwrap(),
    ]
}

#[test]
fn mixed_problem_has_expected_kinds() {
    let problem = mixed_problem();
    assert_eq!(problem.len(), 5);

    let continuous_count = problem
        .iter()
        .filter(|v| matches!(v.kind(), VariableKind::Continuous { .. }))
        .count();
    let integer_count = problem
        .iter()
        .filter(|v| matches!(v.kind(), VariableKind::Integer { .. }))
        .count();
    let discrete_count = problem
        .iter()
        .filter(|v| matches!(v.kind(), VariableKind::Discrete { .. }))
        .count();
    let categorical_count = problem
        .iter()
        .filter(|v| matches!(v.kind(), VariableKind::Categorical { .. }))
        .count();

    assert_eq!(continuous_count, 2);
    assert_eq!(integer_count, 1);
    assert_eq!(discrete_count, 1);
    assert_eq!(categorical_count, 1);
}

#[test]
fn valid_candidate_validates_against_mixed_problem() {
    let problem = mixed_problem();
    let candidate = Candidate::new(vec![vf(3.0), vf(-2.0), vf(7.0), vf(1.5), vf(2.0)]).unwrap();
    assert!(candidate.validate_against(&problem).is_ok());
}

#[test]
fn candidate_arity_mismatch_is_dimension_mismatch() {
    let problem = mixed_problem();
    let candidate = Candidate::new(vec![vf(3.0), vf(-2.0), vf(7.0), vf(1.5)]).unwrap();
    let error = candidate.validate_against(&problem).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DimensionMismatch);
}

#[test]
fn valid_float_rejects_nan_and_infinities() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = ValidFloat::new(bad).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidNotFinite);
    }
    assert!(ValidFloat::new(0.0).is_ok());
}

#[test]
fn inverted_bounds_are_rejected() {
    let error = VariableKind::continuous(vf(5.0), vf(1.0)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvertedBounds);

    let error = VariableKind::integer(5, 1).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvertedBounds);
}

#[test]
fn equal_bounds_are_allowed() {
    assert!(VariableKind::continuous(vf(2.0), vf(2.0)).is_ok());
    assert!(VariableKind::integer(2, 2).is_ok());
}

#[test]
fn empty_discrete_set_and_empty_categorical_are_rejected() {
    let error = DiscreteSet::new(Vec::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);

    let error = VariableKind::categorical(0).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);
}

#[test]
fn non_ascending_or_duplicated_discrete_set_is_rejected() {
    let descending = DiscreteSet::new(vec![vf(2.0), vf(1.0)]).unwrap_err();
    assert_eq!(descending.kind(), ErrorKind::InvertedBounds);

    let duplicated = DiscreteSet::new(vec![vf(1.0), vf(1.0)]).unwrap_err();
    assert_eq!(duplicated.kind(), ErrorKind::InvertedBounds);
}

#[test]
fn negative_tolerance_and_violation_are_rejected() {
    let error = ConstraintKind::soft(vf(-0.1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvertedBounds);

    let error = ConstraintViolation::new(vf(-0.1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvertedBounds);
}

#[test]
fn candidate_must_not_be_empty() {
    let error = Candidate::new(Vec::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);
}

#[test]
fn objective_values_must_not_be_empty() {
    let error = ObjectiveValues::new(Vec::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);
}

#[test]
fn integer_value_must_be_integral_and_in_range() {
    let variable = Variable::new(VariableId(0), VariableKind::integer(0, 10).unwrap()).unwrap();

    assert!(variable.validate_value(vf(5.0)).is_ok());
    assert!(variable.validate_value(vf(-1.0)).is_err());
    assert!(variable.validate_value(vf(11.0)).is_err());

    let non_integral = variable.validate_value(vf(5.5)).unwrap_err();
    assert_eq!(non_integral.kind(), ErrorKind::DomainMismatch);
}

#[test]
fn discrete_value_must_be_a_member() {
    let variable = Variable::new(
        VariableId(0),
        VariableKind::discrete(DiscreteSet::new(vec![vf(0.5), vf(1.5), vf(2.5)]).unwrap()).unwrap(),
    )
    .unwrap();

    assert!(variable.validate_value(vf(1.5)).is_ok());
    let error = variable.validate_value(vf(2.0)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ValueNotInDiscreteSet);
}

#[test]
fn categorical_value_must_be_integral_and_in_range() {
    let variable = Variable::new(VariableId(0), VariableKind::categorical(3).unwrap()).unwrap();

    assert!(variable.validate_value(vf(0.0)).is_ok());
    assert!(variable.validate_value(vf(2.0)).is_ok());

    let too_high = variable.validate_value(vf(3.0)).unwrap_err();
    assert_eq!(too_high.kind(), ErrorKind::IndexOutOfRange);

    let non_integral = variable.validate_value(vf(1.5)).unwrap_err();
    assert_eq!(non_integral.kind(), ErrorKind::IndexOutOfRange);

    let negative = variable.validate_value(vf(-1.0)).unwrap_err();
    assert_eq!(negative.kind(), ErrorKind::IndexOutOfRange);
}

fn descriptors() -> Vec<ConstraintDescriptor> {
    vec![
        ConstraintDescriptor::new(0, ConstraintKind::Hard).unwrap(),
        ConstraintDescriptor::new(1, ConstraintKind::Hard).unwrap(),
        ConstraintDescriptor::new(2, ConstraintKind::soft(vf(0.1)).unwrap()).unwrap(),
        ConstraintDescriptor::new(3, ConstraintKind::soft(vf(0.0)).unwrap()).unwrap(),
    ]
}

fn violations(values: [f64; 4]) -> Vec<ConstraintViolation> {
    values
        .into_iter()
        .map(|value| ConstraintViolation::new(vf(value)).unwrap())
        .collect()
}

#[test]
fn feasibility_matrix() {
    let descriptors = descriptors();

    // All satisfied -> Feasible and admissible.
    let feasible = evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.0, 0.0])).unwrap();
    assert_eq!(feasible, Feasibility::Feasible);
    assert!(feasible.is_admissible());

    // Hard violation -> InfeasibleHard and not admissible.
    let hard = evaluate_feasibility(&descriptors, &violations([0.0, 0.5, 0.0, 0.0])).unwrap();
    assert_eq!(hard, Feasibility::InfeasibleHard { violated: vec![1] });
    assert!(!hard.is_admissible());

    // Soft exactly at tolerance -> still Feasible (inclusive).
    let at_tolerance =
        evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.1, 0.0])).unwrap();
    assert_eq!(at_tolerance, Feasibility::Feasible);
    assert!(at_tolerance.is_admissible());

    // Soft beyond tolerance -> SoftViolation and still admissible.
    let soft = evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.2, 0.0])).unwrap();
    assert_eq!(soft, Feasibility::SoftViolation { violated: vec![2] });
    assert!(soft.is_admissible());

    // Zero-tolerance soft with any positive violation -> SoftViolation.
    let zero_tolerance =
        evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.0, 0.0001])).unwrap();
    assert_eq!(
        zero_tolerance,
        Feasibility::SoftViolation { violated: vec![3] }
    );
    assert!(zero_tolerance.is_admissible());
}

#[test]
fn mixed_hard_and_soft_records_hard_precedence() {
    let descriptors = descriptors();
    // Hard violated at index 0 and soft also exceeded at index 2.
    let mixed = evaluate_feasibility(&descriptors, &violations([1.0, 0.0, 0.5, 0.0])).unwrap();
    assert_eq!(mixed, Feasibility::InfeasibleHard { violated: vec![0] });
    assert!(!mixed.is_admissible());

    // Hard satisfied but soft exceeded -> SoftViolation only.
    let soft_only = evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.9, 0.0])).unwrap();
    assert_eq!(soft_only, Feasibility::SoftViolation { violated: vec![2] });

    // Hard violated with soft within tolerance -> InfeasibleHard.
    let hard_only = evaluate_feasibility(&descriptors, &violations([0.0, 2.0, 0.05, 0.0])).unwrap();
    assert_eq!(hard_only, Feasibility::InfeasibleHard { violated: vec![1] });
}

#[test]
fn feasibility_dimension_mismatch_is_reported() {
    let descriptors = descriptors();
    let error =
        evaluate_feasibility(&descriptors, &violations([0.0, 0.0, 0.0, 0.0])[..3]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DimensionMismatch);
}

#[test]
fn evaluation_outcome_failure_round_trips_typed_error() {
    let original = EvaluationOutcome::Failure {
        error: MathError::new(ErrorKind::DomainMismatch, "candidate arity mismatch")
            .with_context("objective 1"),
    };

    let bytes = encode_evaluation_outcome(&original).unwrap();
    let decoded = decode_evaluation_outcome(&bytes).unwrap();
    assert_eq!(decoded, original);

    match decoded {
        EvaluationOutcome::Failure { error } => {
            assert_eq!(error.kind(), ErrorKind::DomainMismatch);
            assert_eq!(error.message(), "candidate arity mismatch");
            assert_eq!(error.context(), &["objective 1".to_string()]);
        }
        EvaluationOutcome::Success { .. } => panic!("expected a typed failure"),
    }
}

// Integer bounds are i64 and cardinality is u64; both stop being exactly
// representable as f64 at |value| >= 2^53, so the domain checks must compare in
// exact integer space. These regressions pin that behavior.

#[test]
fn integer_bounds_compare_in_exact_integer_space() {
    // i64::MAX is 2^63 - 1 and rounds up to 2^63 when converted to f64, so the
    // old f64 comparison wrongly accepted 2^63 as "within bounds".
    let wide = Variable::new(VariableId(0), VariableKind::integer(0, i64::MAX).unwrap()).unwrap();

    let two_pow_63 = ValidFloat::new(9_223_372_036_854_775_808.0).unwrap();
    assert_eq!(
        wide.validate_value(two_pow_63).unwrap_err().kind(),
        ErrorKind::DomainMismatch
    );

    // 2^63 - 1024 is the largest f64 strictly below 2^63; it is integral and
    // <= i64::MAX, so it must be accepted.
    let largest_below = ValidFloat::new(9_223_372_036_854_775_808.0 - 1024.0).unwrap();
    assert!(wide.validate_value(largest_below).is_ok());

    // A lower bound above 2^53 (2^53 + 1, which itself rounds to 2^53 as f64):
    // the value 2^53 must be rejected as below the bound...
    let shifted = Variable::new(
        VariableId(1),
        VariableKind::integer(9_007_199_254_740_993, 18_014_398_509_481_984).unwrap(),
    )
    .unwrap();
    let two_pow_53 = ValidFloat::new(9_007_199_254_740_992.0).unwrap();
    assert_eq!(
        shifted.validate_value(two_pow_53).unwrap_err().kind(),
        ErrorKind::DomainMismatch
    );

    // ...while 2^53 + 2 (exactly representable and >= the bound) is accepted.
    let two_pow_53_plus_two = ValidFloat::new(9_007_199_254_740_994.0).unwrap();
    assert!(shifted.validate_value(two_pow_53_plus_two).is_ok());
}

#[test]
fn categorical_bounds_compare_in_exact_integer_space() {
    // Cardinality 2^53 + 1 rounds down to 2^53 as f64, so the old comparison
    // wrongly rejected the valid index 2^53.
    let big = Variable::new(
        VariableId(0),
        VariableKind::categorical(9_007_199_254_740_993).unwrap(),
    )
    .unwrap();
    let two_pow_53 = ValidFloat::new(9_007_199_254_740_992.0).unwrap();
    assert!(big.validate_value(two_pow_53).is_ok());

    // The smallest representable f64 >= cardinality (2^53 + 2) is rejected.
    let two_pow_53_plus_two = ValidFloat::new(9_007_199_254_740_994.0).unwrap();
    assert_eq!(
        big.validate_value(two_pow_53_plus_two).unwrap_err().kind(),
        ErrorKind::IndexOutOfRange
    );

    let max = Variable::new(VariableId(1), VariableKind::categorical(u64::MAX).unwrap()).unwrap();
    // 2^64 overflows u64 and is rejected; 2^63 fits and is accepted.
    let two_pow_64 = ValidFloat::new(18_446_744_073_709_551_616.0).unwrap();
    assert_eq!(
        max.validate_value(two_pow_64).unwrap_err().kind(),
        ErrorKind::IndexOutOfRange
    );
    let two_pow_63 = ValidFloat::new(9_223_372_036_854_775_808.0).unwrap();
    assert!(max.validate_value(two_pow_63).is_ok());
}

//! Evidence that the core contracts are domain-agnostic, dependency-free and
//! compose with an arbitrary application model without invoking a solver.

use mamut_math_core::{Candidate, ErrorKind, MathError, ObjectiveValues, ValidFloat};

/// The crate manifest must not declare any third-party dependency at all.
#[test]
fn no_third_party_dependencies() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest must be readable");

    assert!(
        !manifest.contains("[dependencies]"),
        "core crate must not declare a [dependencies] table:\n{manifest}"
    );
    assert!(
        !manifest.contains("[dev-dependencies]"),
        "core crate must not declare a [dev-dependencies] table:\n{manifest}"
    );
}

/// A zero-domain-vocabulary model that composes with the public API.
///
/// It proves the contract layer works with an arbitrary application type and
/// that no solver is invoked from core: the model owns the arithmetic.
struct DemoModel {
    coeffs: [f64; 3],
    w: [f64; 2],
}

impl DemoModel {
    fn evaluate(&self, candidate: &Candidate) -> Result<ObjectiveValues, MathError> {
        let x = candidate.as_slice();
        if x.len() != self.coeffs.len() + self.w.len() {
            return Err(MathError::new(
                ErrorKind::DimensionMismatch,
                "unexpected candidate arity",
            ));
        }

        let f1 =
            self.coeffs[0] * x[0].get() + self.coeffs[1] * x[1].get() + self.coeffs[2] * x[2].get();
        let f2 = self.w[0] * x[3].get() + self.w[1] * x[4].get();

        ObjectiveValues::new(vec![ValidFloat::new(f1)?, ValidFloat::new(f2)?])
    }
}

#[test]
fn public_api_composes_with_an_app_model() {
    let model = DemoModel {
        coeffs: [2.0, 3.0, 4.0],
        w: [1.0, -1.0],
    };
    let candidate = Candidate::new(vec![
        ValidFloat::new(1.0).unwrap(),
        ValidFloat::new(2.0).unwrap(),
        ValidFloat::new(3.0).unwrap(),
        ValidFloat::new(5.0).unwrap(),
        ValidFloat::new(2.0).unwrap(),
    ])
    .unwrap();

    let objectives = model.evaluate(&candidate).unwrap();
    // f1 = 2*1 + 3*2 + 4*3 = 20, f2 = 1*5 + (-1)*2 = 3.
    assert_eq!(objectives.get(0), Some(&ValidFloat::new(20.0).unwrap()));
    assert_eq!(objectives.get(1), Some(&ValidFloat::new(3.0).unwrap()));

    // Wrong arity is a typed failure, never a silent sentinel.
    let short = Candidate::new(vec![ValidFloat::new(1.0).unwrap()]).unwrap();
    let error = model.evaluate(&short).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DimensionMismatch);
}

fn takes_dyn(_: &dyn std::error::Error) {}

fn assert_copy<T: Copy>() {}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn math_error_is_a_standard_error_object() {
    let error = MathError::new(ErrorKind::EmptyCollection, "empty");
    takes_dyn(&error);
    assert_send_sync::<MathError>();
}

#[test]
fn valid_float_is_copy_send_sync() {
    assert_copy::<ValidFloat>();
    assert_send_sync::<ValidFloat>();
}

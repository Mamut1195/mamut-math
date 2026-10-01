#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! # MAMUT Math — Core mathematical contracts (v0.1)
//!
//! Domain-agnostic contract layer consumed by every MAMUT Math optimizer and
//! sensitivity engine. It defines *what* a mathematical optimization problem is
//! (variables, candidates, objectives, constraints, evaluations) without any
//! domain vocabulary and without invoking any solver.
//!
//! ## Scope and invariants
//! - `f64` is the canonical scalar for V0.x (no generic `Scalar` parameter).
//! - Third-party types never appear in public signatures; the crate has **no
//!   external dependencies at all**.
//! - Values are validated at construction: `NaN`/`±Inf` and inverted bounds are
//!   rejected with a typed [`MathError`].
//! - Serialization (see [`codec`]) is canonical, versioned and deterministic.
//!
//! ## Evidence manifests
//! Run manifests and reproducibility evidence (`RunManifest`, `Fingerprint`,
//! `StopReason`, `EvaluationRecord`, all in [`evidence`]) are generic,
//! backend-neutral contracts owned by this crate: they record run identity,
//! problem fingerprints, solver/backend versions, seed metadata, configs,
//! evaluation counts, stop reasons, precision, targets, diagnostics and artifact
//! hashes without embedding domain physics, normative logic, or secret material.
//! Random-number generation and execution ports (`RngPort`/PRNG,
//! checkpoint/replay) remain out of scope and will be introduced by their own
//! tickets (`MATH-RNG-001` and followers); evidence types never reference them.

pub mod codec;
pub mod constraint;
pub mod error;
pub mod evaluation;
pub mod evidence;
pub mod objective;
pub mod value;
pub mod variable;

pub use codec::{
    decode_candidate, decode_constraint_violations, decode_diagnostics, decode_evaluation_outcome,
    decode_feasibility, decode_objective_values, decode_variables, encode_candidate,
    encode_constraint_violations, encode_diagnostics, encode_evaluation_outcome,
    encode_feasibility, encode_objective_values, encode_variables, SCHEMA_VERSION,
};
pub use constraint::{
    evaluate_feasibility, ConstraintDescriptor, ConstraintKind, ConstraintViolation,
};
pub use error::{ErrorKind, MathError};
pub use evaluation::{
    Candidate, Diagnostic, DiagnosticCode, Diagnostics, EvaluationOutcome, Feasibility,
};
pub use objective::{ObjectiveDescriptor, ObjectiveSense, ObjectiveValues};
pub use value::ValidFloat;
pub use variable::{DiscreteSet, Variable, VariableId, VariableKind};

pub use codec::{
    decode_constraint_descriptors, decode_objective_descriptors, decode_run_manifest,
    encode_constraint_descriptors, encode_objective_descriptors, encode_run_manifest,
};
pub use evidence::{
    ArtifactHash, ConfigParam, ConfigValue, EvaluationRecord, EvidenceExport, Fingerprint,
    Precision, RngSpec, RunId, RunKind, RunManifest, RunManifestParams, SolverVersion, StopReason,
    FINGERPRINT_ALGORITHM_SHA256_V1, MANIFEST_VERSION,
};

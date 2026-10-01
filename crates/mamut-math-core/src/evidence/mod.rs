//! Reproducible evidence for numerical runs.
//!
//! Every optimizer or sensitivity run — whatever backend executes it — can
//! produce one backend-neutral [`RunManifest`]: run identity, problem
//! fingerprint, solver/backend versions, deterministic-vs-stochastic run kind,
//! configuration parameters, precision, target, evaluation accounting, terminal
//! [`StopReason`], diagnostics and artifact hashes.
//!
//! ## Non-goals
//!
//! This module owns no filesystem or database storage and no domain payloads.
//! It records only metadata; secret material is never represented, and the
//! failure/abort/unknown stop reasons carry no message. Random-number generation
//! is deliberately absent: a stochastic run stores its seed and RNG identity as
//! plain data ([`RunKind`]/[`RngSpec`]) and never references an RNG port.
//!
//! ## Canonical serialization
//!
//! The manifest has both a canonical binary encoding (see
//! [`crate::codec::encode_run_manifest`]) and a canonical JSON encoding (see
//! [`EvidenceExport`]). Both are deterministic and byte-stable for a given value.

mod artifact;
mod config;
mod digest;
mod evaluation_record;
mod export;
mod fingerprint;
mod manifest;
mod precision;
mod run_id;
mod solver;
mod stop;

pub use artifact::ArtifactHash;
pub use config::{ConfigParam, ConfigValue};
pub use evaluation_record::EvaluationRecord;
pub use export::EvidenceExport;
pub use fingerprint::Fingerprint;
pub use manifest::{
    RunManifest, RunManifestParams, FINGERPRINT_ALGORITHM_SHA256_V1, MANIFEST_VERSION,
};
pub use precision::Precision;
pub use run_id::RunId;
pub use solver::{RngSpec, RunKind, SolverVersion};
pub use stop::StopReason;

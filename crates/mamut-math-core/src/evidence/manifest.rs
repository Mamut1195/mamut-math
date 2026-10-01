//! The reproducible run manifest and its canonical wire encoding.

use crate::codec::{decode_values, write_count, write_u32, write_u64, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};
use crate::evaluation::Diagnostics;
use crate::value::ValidFloat;

use super::artifact::ArtifactHash;
use super::config::ConfigParam;
use super::evaluation_record::EvaluationRecord;
use super::fingerprint::Fingerprint;
use super::precision::Precision;
use super::run_id::RunId;
use super::solver::{read_u128, RunKind, SolverVersion};
use super::stop::StopReason;

/// Version of the run-manifest schema.
pub const MANIFEST_VERSION: u32 = 1;

/// Algorithm label used by [`Fingerprint::for_problem`].
pub const FINGERPRINT_ALGORITHM_SHA256_V1: &str = "sha256-v1";

/// Inputs to [`RunManifest::new`].
///
/// This is a plain data carrier: it performs no validation itself. Every
/// collection invariant is enforced by [`RunManifest::new`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunManifestParams {
    /// The manifest schema version; must equal [`MANIFEST_VERSION`].
    pub manifest_version: u32,
    /// The run identifier.
    pub run_id: RunId,
    /// The fingerprint of the problem that was solved.
    pub fingerprint: Fingerprint,
    /// The solver/backend identities and versions that participated.
    pub solver_versions: Vec<SolverVersion>,
    /// Whether the run was deterministic or stochastic.
    pub run_kind: RunKind,
    /// The configuration parameters supplied to the run.
    pub params: Vec<ConfigParam>,
    /// The numerical tolerances the run was configured with.
    pub precision: Precision,
    /// The optional target values the run aimed for.
    pub target: Option<Vec<ValidFloat>>,
    /// Evaluation/effort accounting.
    pub evaluation: EvaluationRecord,
    /// The terminal state of the run.
    pub stop_reason: StopReason,
    /// Typed diagnostics raised during the run.
    pub diagnostics: Diagnostics,
    /// Hashes of artifacts produced by the run.
    pub artifacts: Vec<ArtifactHash>,
}

/// A validated, reproducible evidence record for one numerical run.
///
/// Construct with [`RunManifest::new`] and read with the accessors; the fields
/// are private so collection invariants cannot be bypassed. The canonical wire
/// encoding is produced by
/// [`encode_run_manifest`](crate::codec::encode_run_manifest) and the canonical
/// JSON by [`EvidenceExport`](super::EvidenceExport).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunManifest {
    manifest_version: u32,
    run_id: RunId,
    fingerprint: Fingerprint,
    solver_versions: Vec<SolverVersion>,
    run_kind: RunKind,
    params: Vec<ConfigParam>,
    precision: Precision,
    target: Option<Vec<ValidFloat>>,
    evaluation: EvaluationRecord,
    stop_reason: StopReason,
    diagnostics: Diagnostics,
    artifacts: Vec<ArtifactHash>,
}

impl RunManifest {
    /// Validate `params` and build a manifest.
    ///
    /// Checks run in this order and short-circuit on the first failure:
    /// 1. `manifest_version` must equal [`MANIFEST_VERSION`]
    ///    ([`ErrorKind::UnsupportedSchemaVersion`]);
    /// 2. `solver_versions` must be non-empty ([`ErrorKind::EmptyCollection`]);
    /// 3. solver ids must be unique, case-sensitively ([`ErrorKind::Reserved`]);
    /// 4. config parameter names must be unique, case-sensitively
    ///    ([`ErrorKind::Reserved`]);
    /// 5. `target`, when `Some`, must be non-empty ([`ErrorKind::EmptyCollection`]);
    /// 6. artifact names must be unique, case-sensitively ([`ErrorKind::Reserved`]).
    ///
    /// # Errors
    /// See the ordered list above.
    pub fn new(params: RunManifestParams) -> Result<Self, MathError> {
        if params.manifest_version != MANIFEST_VERSION {
            return Err(MathError::new(
                ErrorKind::UnsupportedSchemaVersion,
                "unsupported manifest version",
            ));
        }
        if params.solver_versions.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "run manifest needs at least one solver version",
            ));
        }
        if has_duplicates(params.solver_versions.iter().map(SolverVersion::id)) {
            return Err(MathError::new(
                ErrorKind::Reserved,
                "solver version ids must be unique",
            ));
        }
        if has_duplicates(params.params.iter().map(ConfigParam::name)) {
            return Err(MathError::new(
                ErrorKind::Reserved,
                "config parameter names must be unique",
            ));
        }
        if let Some(target) = &params.target {
            if target.is_empty() {
                return Err(MathError::new(
                    ErrorKind::EmptyCollection,
                    "target must not be empty when present",
                ));
            }
        }
        if has_duplicates(params.artifacts.iter().map(ArtifactHash::name)) {
            return Err(MathError::new(
                ErrorKind::Reserved,
                "artifact names must be unique",
            ));
        }

        Ok(Self {
            manifest_version: params.manifest_version,
            run_id: params.run_id,
            fingerprint: params.fingerprint,
            solver_versions: params.solver_versions,
            run_kind: params.run_kind,
            params: params.params,
            precision: params.precision,
            target: params.target,
            evaluation: params.evaluation,
            stop_reason: params.stop_reason,
            diagnostics: params.diagnostics,
            artifacts: params.artifacts,
        })
    }

    /// The manifest schema version.
    #[must_use]
    pub fn manifest_version(&self) -> u32 {
        self.manifest_version
    }

    /// The run identifier.
    #[must_use]
    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    /// The problem fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    /// The solver/backend versions.
    #[must_use]
    pub fn solver_versions(&self) -> &[SolverVersion] {
        &self.solver_versions
    }

    /// Whether the run was deterministic or stochastic.
    #[must_use]
    pub fn run_kind(&self) -> &RunKind {
        &self.run_kind
    }

    /// The configuration parameters.
    #[must_use]
    pub fn params(&self) -> &[ConfigParam] {
        &self.params
    }

    /// The configured numerical tolerances.
    #[must_use]
    pub fn precision(&self) -> Precision {
        self.precision
    }

    /// The target values, if the run had a target.
    #[must_use]
    pub fn target(&self) -> Option<&[ValidFloat]> {
        self.target.as_deref()
    }

    /// The evaluation/effort accounting.
    #[must_use]
    pub fn evaluation(&self) -> EvaluationRecord {
        self.evaluation
    }

    /// The terminal state of the run.
    #[must_use]
    pub fn stop_reason(&self) -> StopReason {
        self.stop_reason
    }

    /// The diagnostics raised during the run.
    #[must_use]
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The artifact hashes.
    #[must_use]
    pub fn artifacts(&self) -> &[ArtifactHash] {
        &self.artifacts
    }
}

/// Whether a sequence of case-sensitive names contains a duplicate.
fn has_duplicates<'a>(names: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen: Vec<&str> = Vec::new();
    for name in names {
        if seen.contains(&name) {
            return true;
        }
        seen.push(name);
    }
    false
}

impl Canonical for RunManifest {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u32(self.manifest_version, out);
        out.extend_from_slice(&self.run_id.value().to_le_bytes());
        self.fingerprint.encode(out)?;
        write_count(self.solver_versions.len(), out)?;
        for solver in &self.solver_versions {
            solver.encode(out)?;
        }
        self.run_kind.encode(out)?;
        write_count(self.params.len(), out)?;
        for param in &self.params {
            param.encode(out)?;
        }
        write_u64(self.precision.absolute_tolerance().get().to_bits(), out);
        write_u64(self.precision.relative_tolerance().get().to_bits(), out);
        match &self.target {
            None => out.push(0),
            Some(values) => {
                out.push(1);
                write_count(values.len(), out)?;
                for value in values {
                    write_u64(value.get().to_bits(), out);
                }
            }
        }
        self.evaluation.encode(out)?;
        self.stop_reason.encode(out)?;
        self.diagnostics.encode(out)?;
        write_count(self.artifacts.len(), out)?;
        for artifact in &self.artifacts {
            artifact.encode(out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let manifest_version = dec.read_u32()?;
        let run_id = RunId::new(read_u128(dec)?);
        let fingerprint = Fingerprint::decode(dec)?;
        let solver_versions = decode_values::<SolverVersion>(
            dec,
            true,
            "run manifest needs at least one solver version",
        )?;
        let run_kind = RunKind::decode(dec)?;
        let params = decode_values::<ConfigParam>(dec, false, "params")?;
        let absolute_tolerance = dec.read_valid_float()?;
        let relative_tolerance = dec.read_valid_float()?;
        let precision = Precision::new(absolute_tolerance, relative_tolerance)?;
        let target = match dec.read_u8()? {
            0 => None,
            1 => Some(decode_values::<ValidFloat>(
                dec,
                true,
                "target must not be empty",
            )?),
            _ => {
                return Err(MathError::new(
                    ErrorKind::Reserved,
                    "unknown target option tag",
                ))
            }
        };
        let evaluation = EvaluationRecord::decode(dec)?;
        let stop_reason = StopReason::decode(dec)?;
        let diagnostics = Diagnostics::decode(dec)?;
        let artifacts = decode_values::<ArtifactHash>(dec, false, "artifacts")?;

        RunManifest::new(RunManifestParams {
            manifest_version,
            run_id,
            fingerprint,
            solver_versions,
            run_kind,
            params,
            precision,
            target,
            evaluation,
            stop_reason,
            diagnostics,
            artifacts,
        })
    }
}

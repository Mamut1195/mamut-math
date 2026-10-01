//! Problem fingerprints: a deterministic digest of an optimization problem's
//! structural contract (variables, objectives, constraints).

use crate::codec::{write_count, write_string, Canonical, Decoder};
use crate::constraint::ConstraintDescriptor;
use crate::error::{ErrorKind, MathError};
use crate::objective::ObjectiveDescriptor;
use crate::variable::Variable;

/// ASCII domain tag prefixed to the canonical problem byte stream.
const PROBLEM_DOMAIN: &[u8] = b"MAMUT-MATH-PROBLEM-V1";

/// Algorithm label produced by [`Fingerprint::for_problem`] and
/// [`Fingerprint::sha256_v1`].
const SHA256_V1: &str = "sha256-v1";

/// A deterministic digest of a problem or an arbitrary artifact.
///
/// The `algorithm` labels how `digest` was produced (currently always
/// `"sha256-v1"`). The digest covers only structural contract data — never
/// domain payloads, secret material or solver state.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    algorithm: String,
    digest: [u8; 32],
}

impl Fingerprint {
    /// Build a fingerprint from an algorithm label and a 32-byte digest.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `algorithm` is empty.
    pub fn new(algorithm: String, digest: [u8; 32]) -> Result<Self, MathError> {
        if algorithm.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "fingerprint algorithm must not be empty",
            ));
        }
        Ok(Self { algorithm, digest })
    }

    /// Build a fingerprint for a pre-computed SHA-256 digest.
    ///
    /// The algorithm label is fixed to `"sha256-v1"`.
    #[must_use]
    pub fn sha256_v1(digest: [u8; 32]) -> Self {
        Self {
            algorithm: SHA256_V1.to_string(),
            digest,
        }
    }

    /// Fingerprint a problem from its structural contract.
    ///
    /// The digest is the SHA-256 of the canonical problem byte stream:
    ///
    /// ```text
    /// "MAMUT-MATH-PROBLEM-V1"
    /// u32 LE variable count, then per variable: u64 LE id + kind payload
    /// u32 LE objective count, then per objective: u32 LE index + u8 sense tag
    /// u32 LE constraint count, then per constraint: u32 LE index + kind payload
    /// ```
    ///
    /// Empty objective and constraint sections are allowed (a problem may have
    /// no constraints).
    ///
    /// # Errors
    /// [`ErrorKind::IndexOutOfRange`] if any length or index exceeds the `u32`
    /// wire-format range.
    pub fn for_problem(
        variables: &[Variable],
        objectives: &[ObjectiveDescriptor],
        constraints: &[ConstraintDescriptor],
    ) -> Result<Self, MathError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PROBLEM_DOMAIN);
        write_count(variables.len(), &mut bytes)?;
        for variable in variables {
            Canonical::encode(variable, &mut bytes)?;
        }
        write_count(objectives.len(), &mut bytes)?;
        for objective in objectives {
            Canonical::encode(objective, &mut bytes)?;
        }
        write_count(constraints.len(), &mut bytes)?;
        for constraint in constraints {
            Canonical::encode(constraint, &mut bytes)?;
        }
        Ok(Self::sha256_v1(super::digest::sha256(&bytes)))
    }

    /// The algorithm label.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// The 32-byte digest.
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

impl Canonical for Fingerprint {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_string(&self.algorithm, out)?;
        out.extend_from_slice(&self.digest);
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let algorithm = dec.read_string()?;
        let digest = read_digest(dec)?;
        Fingerprint::new(algorithm, digest)
    }
}

/// Read 32 raw digest bytes as four little-endian `u64` words.
///
/// The wire format stores the digest verbatim; reading and re-emitting four LE
/// words is byte-identical and avoids needing a slice reader.
pub(crate) fn read_digest(dec: &mut Decoder<'_>) -> Result<[u8; 32], MathError> {
    let mut digest = [0u8; 32];
    for chunk in digest.chunks_exact_mut(8) {
        chunk.copy_from_slice(&dec.read_u64()?.to_le_bytes());
    }
    Ok(digest)
}
